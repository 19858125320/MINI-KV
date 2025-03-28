use crossbeam_skiplist::SkipMap;
use bincode::{self,Encode,Decode,enc::write::Writer,de::read::Reader};
use log::info;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{atomic::{AtomicU64, Ordering},Arc, Mutex};
use std::cell::RefCell;
use std::ffi::OsStr;
use std::result::Result as stdResult;
use serde::{Deserialize, Serialize};
use crate::{Result,KvsError,KVEngine};

const COMPACTION_THRESHOLD: u64 = 1024 * 1024 * 1024;//1GB

#[derive(Clone)]
pub struct KvStore {
    // directory for the log and other data.
    path: Arc<PathBuf>,
    // reader of the current log.
    reader: KvStoreReader,
    // writer of the current log.
    writer: Arc<Mutex<KvStoreWriter>>,
    // the generation number of the current log.
    index: Arc<SkipMap<String, CommandPos>>,
}

impl KvStore{
    /// Opens a `KvStore` with the given path.
    ///
    /// This will create a new directory if the given one does not exist.
    ///
    /// # Errors
    ///
    /// It propagates I/O or deserialization errors during the log replay.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = Arc::new(path.into());
        fs::create_dir_all(&*path)?;

        let mut readers = BTreeMap::new();
        let index = Arc::new(SkipMap::new());

        let gen_list = sorted_gen_list(&path)?;
        let mut uncompacted = 0;

        for &r#gen in &gen_list {
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, r#gen))?)?;
            uncompacted += load(r#gen, &mut reader, &*index)?;
            readers.insert(r#gen, reader);
        }
       
        let mut current_gen:u64=0;
        if gen_list.len()==0{
            current_gen = 1;
        }else{
            current_gen = gen_list.last().cloned().unwrap();
        }

        let writer = new_log_file(&path, current_gen)?;
        let safe_point = Arc::new(AtomicU64::new(0));

        let reader = KvStoreReader {
            path: Arc::clone(&path),
            safe_point,
            readers: RefCell::new(readers),
        };

        let writer = KvStoreWriter {
            reader: reader.clone(),
            writer,
            current_gen,
            uncompacted,
            path: Arc::clone(&path),
            index: Arc::clone(&index),
        };
       
        Ok(KvStore {
            path,
            reader,
            writer:Arc::new(Mutex::new(writer)),
            index:index,
        })
    }
}

impl KVEngine for KvStore {

    /// Sets the value of a string key to a string.
    ///
    /// If the key already exists, the previous value will be overwritten.
    ///
    /// # Errors
    ///
    /// It propagates I/O or serialization errors during writing the log.
    fn set(&self, key: String, value: String) -> Result<()> {
        self.writer.lock().unwrap().set(key, value)
    }

    /// Gets the string value of a given string key.
    ///
    /// Returns `None` if the given key does not exist.
    ///
    /// # Errors
    ///
    /// It returns `KvsError::UnexpectedCommandType` if the given command type unexpected.
    fn get(&self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.index.get(&key) {
            if let Command::Set { value, .. } = self.reader.read_command(*cmd_pos.value())? {
                Ok(Some(value))
            } else {
                Err(KvsError::UnexpectedCommandType)
            }
        } else {
            Ok(None)
        }
    }

    fn scan(&self, start: String,end:String) -> Result<Vec<String>> {
        let mut res=Vec::new();
        for entry in self.index.range(start..=end){
            if let Command::Set { value, .. } = self.reader.read_command(*entry.value())? {
                res.push(value);
            } else {
                return Err(KvsError::UnexpectedCommandType);
            }
        }
        Ok(res)
    }
    /// Removes a given key.
    ///
    /// # Errors
    ///
    /// It returns `KvsError::KeyNotFound` if the given key is not found.
    ///
    /// It propagates I/O or serialization errors during writing the log.
    fn remove(&self, key: String) -> Result<()> {
        self.writer.lock().unwrap().remove(key)
    }
}

/// A single thread reader.
///
/// Each `KvStore` instance has its own `KvStoreReader` and
/// `KvStoreReader`s open the same files separately. So the user
/// can read concurrently through multiple `KvStore`s in different
/// threads.
struct KvStoreReader {
    path: Arc<PathBuf>,
    // generation of the latest compaction file
    safe_point: Arc<AtomicU64>,
    readers: RefCell<BTreeMap<u64, BufReaderWithPos<File>>>,
}

impl KvStoreReader {
    /// Close file handles with generation number less than safe_point.
    ///
    /// `safe_point` is updated to the latest compaction gen after a compaction finishes.
    /// The compaction generation contains the sum of all operations before it and the
    /// in-memory index contains no entries with generation number less than safe_point.
    /// So we can safely close those file handles and the stale files can be deleted.
    fn close_stale_handles(&self) {
        let mut readers = self.readers.borrow_mut();
        while !readers.is_empty() {
            let first_gen = *readers.keys().next().unwrap();
            if self.safe_point.load(Ordering::SeqCst) <= first_gen{
                break;
            }
            readers.remove(&first_gen);
        }
    }

    /// Read the log file at the given `CommandPos`.
    fn read_and<F,R>(&self, cmd_pos: CommandPos,f:F) -> Result<R>
    where
        F: FnOnce(&mut BufReaderWithPos<File>) -> Result<R>,
    {
        self.close_stale_handles();

        let mut readers = self.readers.borrow_mut();
        // Open the file if we haven't opened it in this `KvStoreReader`.
        // We don't use entry API here because we want the errors to be propogated.
        if !readers.contains_key(&cmd_pos.r#gen) {
            let reader = BufReaderWithPos::new(File::open(log_path(&self.path, cmd_pos.r#gen))?)?;
            readers.insert(cmd_pos.r#gen, reader);
        }
        let reader = readers.get_mut(&cmd_pos.r#gen).unwrap();
        reader.seek(SeekFrom::Start(cmd_pos.pos))?;
        f(reader)
    }

    // Read the log file at the given `CommandPos` and deserialize it to `Command`.
    fn read_command(&self, cmd_pos: CommandPos) -> Result<Command> {
        self.read_and(cmd_pos, |cmd_reader| {
            cmd_reader.take(cmd_pos.len);
            let mut buf=vec![0u8;cmd_pos.len as usize];
            cmd_reader.read_exact(&mut buf)?;
            let res:(Command,usize)=bincode::decode_from_slice(buf.as_slice(), bincode::config::standard())?;
            Ok(res.0)
        })
    }
}

impl Clone for KvStoreReader {
    fn clone(&self) -> KvStoreReader {
        KvStoreReader {
            path: Arc::clone(&self.path),
            safe_point: Arc::clone(&self.safe_point),
            // don't use other KvStoreReader's readers
            //readers: RefCell::new(BTreeMap::new()),
            readers: RefCell::new(BTreeMap::new()),
        }
    }
}

struct KvStoreWriter {
    reader: KvStoreReader,
    writer: BufWriterWithPos<File>,
    current_gen: u64,
    // the number of bytes representing "stale" commands that could be
    // deleted during a compaction
    uncompacted: u64,
    path: Arc<PathBuf>,
    index: Arc<SkipMap<String, CommandPos>>,
}

impl KvStoreWriter {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let cmd = Command::set(key, value);
        let pos = self.writer.pos;
        bincode::encode_into_writer(&cmd, &mut self.writer, bincode::config::standard())?;
        self.writer.flush()?;
        if let Command::Set { key, .. } = cmd {
            if let Some(old_cmd) = self.index.get(&key) {
                self.uncompacted += old_cmd.value().len;
            }
            self.index
                .insert(key.clone(), (self.current_gen, pos..self.writer.pos).into());
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }
        Ok(())
    }

    fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let cmd = Command::remove(key);
            let pos = self.writer.pos;
            bincode::encode_into_writer(&cmd,&mut self.writer,bincode::config::standard())?;
            self.writer.flush()?;
            if let Command::Remove { key } = cmd {
                let old_cmd = self.index.remove(&key).expect("key not found");
                self.uncompacted += old_cmd.value().len;
                // the "remove" command itself can be deleted in the next compaction
                // so we add its length to `uncompacted`
                self.uncompacted += self.writer.pos - pos;
            }

            if self.uncompacted > COMPACTION_THRESHOLD {
                self.compact()?;
            }
            Ok(())
        } else {
            Err(KvsError::KeyNotFound)
        }
    }

    /// Clears stale entries in the log.
    fn compact(&mut self) -> Result<()> {
        // increase current gen by 2. current_gen + 1 is for the compaction file
        let compaction_gen = self.current_gen + 1;
        self.current_gen += 2;
        self.writer = new_log_file(&self.path, self.current_gen)?;

        let mut compaction_writer = new_log_file(&self.path, compaction_gen)?;

        let mut new_pos = 0; // pos in the new log file
        for entry in &mut self.index.iter() {
            let cmd_pos=entry.value();
            let len=self.reader.read_and(*cmd_pos, |reader|{
                let mut buf:Vec<u8>=Vec::with_capacity(cmd_pos.len as usize);
                reader.reader.read_exact(buf.as_mut_slice())?;
                reader.pos+=cmd_pos.len;
                let len=<BufWriterWithPos<File> as std::io::Write>::write(&mut compaction_writer,buf.as_slice())?;
                Ok(len)
            })?;
            self.index.insert(entry.key().clone(), (compaction_gen, new_pos..new_pos + len as u64).into());
            new_pos += len as u64;
        }

        compaction_writer.flush()?;

        self.reader
            .safe_point
            .store(compaction_gen, Ordering::SeqCst);
        self.reader.close_stale_handles();

        // remove stale log files
        // Note that actually these files are not deleted immediately because `KvStoreReader`s
        // still keep open file handles. When `KvStoreReader` is used next time, it will clear
        // its stale file handles. On Unix, the files will be deleted after all the handles
        // are closed. On Windows, the deletions below will fail and stale files are expected
        // to be deleted in the next compaction.

        let stale_gens = sorted_gen_list(&self.path)?
            .into_iter()
            .filter(|&r#gen| r#gen < compaction_gen);
        for stale_gen in stale_gens {
            let file_path = log_path(&self.path, stale_gen);
            fs::remove_file(&file_path)?;
        }
        self.uncompacted = 0;

        Ok(())
    }
}

/// Create a new log file with given generation number and add the reader to the readers map.
///
/// Returns the writer to the log.
fn new_log_file(
    path: &Path,
    r#gen: u64
    // readers: &mut HashMap<u64, BufReaderWithPos<File>>,
) -> Result<BufWriterWithPos<File>> {
    let path = log_path(&path, r#gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?,
    )?;
    
    Ok(writer)
}

/// Returns sorted generation numbers in the given directory.
fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gen_list: Vec<u64> = fs::read_dir(&path)?
        .flat_map(|res| -> Result<_> { Ok(res?.path()) })
        .filter(|path| path.is_file() && path.extension() == Some("log".as_ref()))
        .flat_map(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.trim_end_matches(".log"))
                .map(str::parse::<u64>)
        })
        .flatten()
        .collect();
    gen_list.sort_unstable();
    Ok(gen_list)
}

/// Load the whole log file and store value locations in the index map.
///
/// Returns how many bytes can be saved after a compaction.
fn load(
    r#gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &SkipMap<String, CommandPos>,
) -> Result<u64> {
    // To make sure we read from the beginning of the file.
    let mut pos = reader.seek(SeekFrom::Start(0))?;
    
    let command_iter=CommandIterator{reader};
    let mut uncompacted = 0; // number of bytes that can be saved after a compaction.
    
    for cmd_result in command_iter{
        let (cmd,new_pos ) = cmd_result?;
        match cmd {
            Command::Set { key, .. } => {
                if let Some(old_cmd) = index.get(&key) {
                    uncompacted += old_cmd.value().len;
                }
                index.insert(key, (r#gen, pos..new_pos).into());
            }
            Command::Remove { key } => {
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.value().len;
                }
                // the "remove" command itself can be deleted in the next compaction.
                // so we add its length to `uncompacted`.
                uncompacted += new_pos - pos;
            }
        }
        pos = new_pos;
    }
    Ok(uncompacted)
}

fn log_path(dir: &Path, r#gen: u64) -> PathBuf {
    dir.join(format!("{}.log", r#gen))
}

/// Struct representing a command.
#[derive(Serialize, Deserialize, Encode,Decode,Debug)]
enum Command {
    Set { key: String, value: String },
    Remove { key: String },
}

impl Command {
    fn set(key: String, value: String) -> Command {
        Command::Set { key, value }
    }

    fn remove(key: String) -> Command {
        Command::Remove { key }
    }
}

/// Represents the position and length of a json-serialized command in the log.
#[derive(Debug, Clone, Copy)]
struct CommandPos {
    r#gen: u64,
    pos: u64,
    len: u64,
}

impl From<(u64, Range<u64>)> for CommandPos {
    fn from((r#gen, range): (u64, Range<u64>)) -> Self {
        CommandPos {
            r#gen,
            pos: range.start,
            len: range.end - range.start,
        }
    }
}

struct BufReaderWithPos<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
    len:u64,
}

impl<R: Read + Seek> BufReaderWithPos<R> {
    fn new(mut inner: R) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufReaderWithPos {
            reader: BufReader::new(inner),
            pos,
            len:0,
        })
    }

    fn take(&mut self,len:u64){
        self.len=len;
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.reader.read_exact(buf)?;
        self.pos += buf.len() as u64;
        Ok(())
    }
}

impl<R: Read + Seek> Seek for BufReaderWithPos<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.reader.seek(pos)?;
        Ok(self.pos)
    }
}

impl<R: Read + Seek> Reader for BufReaderWithPos<R> {
    fn read(&mut self, bytes: &mut [u8]) -> stdResult<(), bincode::error::DecodeError> {
        self.reader.read_exact(bytes).map_err(|e| bincode::error::DecodeError::Io {
            inner: e,
            additional: bytes.len(),
        })?;
        self.pos += bytes.len() as u64;
        Ok(())
    }
}

// 流式读取所有 Command
struct CommandIterator<'a,R: Read + Seek> {
    reader: &'a mut BufReaderWithPos<R>,
}

impl<'a,R: Read + Seek> Iterator for CommandIterator<'a,R> {
    type Item = stdResult<(Command, u64), KvsError>;

    fn next(&mut self) -> Option<Self::Item> {
        match bincode::decode_from_reader(&mut self.reader, bincode::config::standard()) {
            Ok(cmd) => {
                Some(Ok((cmd, self.reader.pos)))
            },
            Err(e) => match e {
                bincode::error::DecodeError::Io { inner, .. } if inner.kind() == io::ErrorKind::UnexpectedEof => None, // 文件末尾
                _ => Some(Err(e.into())),
            },
        }
    }
}

struct BufWriterWithPos<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl<W: Write + Seek> BufWriterWithPos<W> {
    fn new(mut inner: W) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufWriterWithPos {
            writer: BufWriter::new(inner),
            pos,
        })
    }
}

impl<W: Write + Seek> Write for BufWriterWithPos<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for BufWriterWithPos<W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}

impl<W: Write + Seek> Writer for BufWriterWithPos<W> {
    fn write(&mut self, bytes: &[u8]) -> stdResult<(), bincode::error::EncodeError> {
        let len=self.writer.write(bytes).map_err(|e| bincode::error::EncodeError::Io {
            inner: e,
            index: self.pos as usize, // 提供错误发生的位置
        })?;
        self.pos += len as u64;
        Ok(())
    }
}