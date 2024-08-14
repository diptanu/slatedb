use std::time::Duration;

pub const DEFAULT_READ_OPTIONS: &ReadOptions = &ReadOptions::default();
pub const DEFAULT_WRITE_OPTIONS: &WriteOptions = &WriteOptions::default();

#[allow(dead_code)]
pub const DEFAULT_COMPACTOR_OPTIONS: &CompactorOptions = &CompactorOptions::default();

/// Whether reads see only writes that have been committed durably to the DB.  A
/// write is considered durably committed if all future calls to read are guaranteed
/// to serve the data written by the write, until some later durably committed write
/// updates the same key.
pub enum ReadLevel {
    /// Client reads will only see data that's been committed durably to the DB.
    Commited,

    /// Clients will see all writes, including those not yet durably committed to the
    /// DB.
    Uncommitted,
}

/// Configuration for client read operations. `ReadOptions` is supplied for each
/// read call and controls the behavior of the read.
pub struct ReadOptions {
    /// The read commit level for read operations.
    pub read_level: ReadLevel,
}

impl ReadOptions {
    /// Create a new ReadOptions with `read_level` set to `Commited`.
    const fn default() -> Self {
        Self {
            read_level: ReadLevel::Commited,
        }
    }
}

/// Configuration for client write operations. `WriteOptions` is supplied for each
/// write call and controls the behavior of the write.
pub struct WriteOptions {
    /// Whether `put` calls should block until the write has been durably committed
    /// to the DB.
    pub await_flush: bool,
}

impl WriteOptions {
    /// Create a new `WriteOptions`` with `await_flush` set to `true`.
    const fn default() -> Self {
        Self { await_flush: true }
    }
}

/// Configuration options for the database. These options are set on client startup.
#[derive(Clone)]
pub struct DbOptions {
    /// How frequently to flush the write-ahead log to object storage (in
    /// milliseconds).
    ///
    /// When setting this configuration, users must consider:
    ///
    /// * **Latency**: The higher the flush interval, the longer it will take for
    ///   writes to be committed to object storage. Writers blocking on `put` calls
    ///   will wait longer for the write. Readers reading committed writes will also
    ///   see data later.
    /// * **API cost**: The lower the flush interval, the more frequently PUT calls
    ///   will be made to object storage. This can increase your object storage costs.
    ///
    /// We recommend setting this value based on your cost and latency tolerance. A
    /// 100ms flush interval should result in $130/month in PUT costs on S3 standard.
    ///
    /// Keep in mind that the flush interval does not include the network latency. A
    /// 100ms flush interval will result in a 100ms + the time it takes to send the
    /// bytes to object storage.
    pub flush_ms: usize,

    /// How frequently to poll for new manifest files (in milliseconds). Refreshing
    /// the manifest file allows writers to detect fencing operations and allows
    /// readers to detect newly compacted data.
    ///
    /// **NOTE: SlateDB secondary readers (i.e. non-writer clients) do not currently
    /// read from the WAL. Such readers only read from L0+. The manifest poll intervals
    /// allows such readers to detect new L0+ files.**
    pub manifest_poll_interval: Duration,

    /// Write SSTables with a bloom filter if the number of keys in the SSTable
    /// is greater than or equal to this value. Reads on small SSTables might be
    /// faster without a bloom filter.
    pub min_filter_keys: u32,

    /// The minimum size a memtable needs to be before it is frozen and flushed to
    /// L0 object storage. Writes will still be flushed to the object storage WAL
    /// (based on flush_ms) regardless of this value. Memtable sizes are checked
    /// every `flush_ms` milliseconds.
    ///
    /// When setting this configuration, users must consider:
    ///
    /// * **Recovery time**: The larger the L0 SSTable size threshold, the less
    ///   frequently it will be written. As a result, the more recovery data there
    ///   will be in the WAL if a process restarts.
    /// * **Number of L0 SSTs/SRs**: The smaller the L0 SSTable size threshold, the
    ///   more SSTs and Sorted Runs there will be. L0 SSTables are not range
    ///   partitioned; each is its own sorted table. Similarly, each Sorted Run also
    ///   stores the entire keyspace. As such, reads that don't hit the WAL or memtable
    ///   may need to scan all L0 SSTables and Sorted Runs. The more there are, the
    ///   slower the scan will be.
    /// * **Memory usage**: The larger the L0 SSTable size threshold, the larger the
    ///   unflushed in-memory memtable will grow. This shouldn't be a concern for most
    ///   workloads, but it's worth considering for workloads with very high L0
    ///   SSTable sizes.
    /// * **API cost**: Smaller L0 SSTable sizes will result in more frequent writes
    ///   to object storage. This can increase your object storage costs.
    /// * **Secondary reader latency**: Secondary (non-writer) clients only see L0+
    ///   writes; they don't see WAL writes. Thus, the higher the L0 SSTable size, the
    ///   less frequently they will be written, and the longer it will take for
    ///   secondary readers to see new data.
    pub l0_sst_size_bytes: usize,

    /// Configuration options for the compactor.
    pub compactor_options: Option<CompactorOptions>,
}

/// Options for the compactor.
#[derive(Clone)]
pub struct CompactorOptions {
    /// The interval at which the compactor checks for a new manifest and decides
    /// if a compaction must be scheduled
    pub(crate) poll_interval: Duration,

    /// A compacted SSTable's maximum size (in bytes). If more data needs to be
    /// written to a Sorted Run during a compaction, a new SSTable will be created
    /// in the Sorted Run when this size is exceeded.
    pub(crate) max_sst_size: usize,
}

/// Default options for the compactor. Currently, only a
/// `SizeTieredCompactionScheduler` compaction strategy is implemented.
impl CompactorOptions {
    /// Returns a `CompactorOptions` with a 5 second poll interval and a 1GB max
    /// SSTable size.
    pub const fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(5),
            max_sst_size: 1024 * 1024 * 1024,
        }
    }
}