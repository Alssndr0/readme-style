<div align="center"><!-- readme-style:center -->

# kvlite

An embedded key-value store for Go that survives power loss.

<!-- readme-style:header theme=ml-research -->
![Go 1.22+](https://img.shields.io/badge/go-1.22%2B-00ADD8?logo=go&logoColor=white&style=flat) [![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue?style=flat)](LICENSE)

**[Features](#features)** · **[Installation](#installation)** · **[Quick start](#quick-start)** · **[Configuration](#configuration)** · **[API](#api)** · **[Benchmarks](#benchmarks)**
<!-- /readme-style:header -->

</div><!-- /readme-style:center -->

kvlite keeps your data in a single file, writes through an append-only log, and recovers to the last
committed transaction after a crash. It has no background goroutines you need to manage and no
dependencies outside the standard library.

<!-- readme-style:toc theme=ml-research -->
<details>
<summary><b>Table of Contents</b></summary>

- [Features](#features)
- [Installation](#installation)
- [Quick start](#quick-start)
- [Configuration](#configuration)
  - [Durability](#durability)
  - [Compaction](#compaction)
  - [Encryption](#encryption)
- [API](#api)
  - [Open and Close](#open-and-close)
  - [Get and Put](#get-and-put)
  - [Delete](#delete)
  - [Iterate](#iterate)
  - [Transactions](#transactions)
- [Benchmarks](#benchmarks)
- [FAQ](#faq)
- [Changelog](#changelog)
- [Contributing](#contributing)
- [License](#license)

</details>
<!-- /readme-style:toc -->

## Features

- Single-file storage with an append-only write-ahead log
- Serializable transactions with optimistic concurrency
- Ordered iteration over keys and key prefixes
- Online compaction that never blocks readers
- Optional encryption at rest with AES-GCM

## Installation

```sh
go get github.com/acme/kvlite
```

kvlite needs Go 1.22 or newer.

## Quick start

```go
db, err := kvlite.Open("data.kv", kvlite.Options{})
if err != nil {
    log.Fatal(err)
}
defer db.Close()

if err := db.Put([]byte("greeting"), []byte("hello")); err != nil {
    log.Fatal(err)
}
value, _ := db.Get([]byte("greeting"))
fmt.Println(string(value))
```

## Configuration

Every option has a safe default, so `kvlite.Options{}` is a good start.

| Option | Default | Meaning |
|--------|---------|---------|
| `SyncWrites` | `true` | Call `fsync` before a commit returns |
| `CompactAt` | `0.5` | Compact when this share of the file is garbage |
| `MaxValueSize` | `1 MiB` | Reject larger values |
| `EncryptionKey` | none | 32-byte key for AES-GCM |

### Durability

With `SyncWrites` on, a committed transaction is on disk when `Commit` returns. Turning it off makes
writes several times faster, but a crash can lose the last second of commits. The file itself is
never corrupted either way.

> [!IMPORTANT]
> network file systems may acknowledge `fsync` before data reaches the disk. Keep the
> database on local storage.

### Compaction

The log grows with every write. When garbage passes `CompactAt`, kvlite rewrites live keys into a new
file and swaps it in atomically. Readers keep using the old file until they finish.

```go
db, err := kvlite.Open("data.kv", kvlite.Options{CompactAt: 0.3})
```

### Encryption

Pass a 32-byte key to encrypt values and keys at rest. The key is never written to the file.

> [!CAUTION]
> losing the key makes the data unrecoverable. kvlite cannot help you get it back.

## API

### Open and Close

`Open` creates the file if it does not exist and replays the log. `Close` flushes pending writes and
releases the file lock.

### Get and Put

```go
err := db.Put(key, value)
value, err := db.Get(key)
```

`Get` returns `kvlite.ErrNotFound` for missing keys.

### Delete

```go
err := db.Delete(key)
```

Deleting a missing key is not an error.

### Iterate

```go
it := db.Iterate([]byte("user:"))
for it.Next() {
    fmt.Println(string(it.Key()), string(it.Value()))
}
```

Iterators see a snapshot taken when they were created.

### Transactions

```go
err := db.Update(func(tx *kvlite.Tx) error {
    balance, _ := tx.Get([]byte("balance"))
    return tx.Put([]byte("balance"), next(balance))
})
```

A transaction that conflicts with a concurrent one returns `kvlite.ErrConflict`; retry it.

## Benchmarks

| Operation | kvlite | bbolt | Badger |
|-----------|--------|-------|--------|
| Put (sync) | 41 µs | 58 µs | 47 µs |
| Get | 0.6 µs | 0.9 µs | 1.1 µs |
| Iterate 1M keys | 180 ms | 240 ms | 210 ms |

Measured on a laptop SSD; see `bench/` to reproduce.

## FAQ

### Can several processes open the same file?

No. kvlite takes an exclusive lock; a second `Open` returns `kvlite.ErrLocked`.

### How large can a database get?

Tested up to 500 GB. Keys and values must fit in memory one at a time.

## Changelog

<details><summary>Click to expand</summary><!-- readme-style:collapse -->

### v1.3.0

- Encryption at rest with AES-GCM
- `Iterate` accepts a prefix

### v1.2.0

- Online compaction no longer blocks readers
- 20 % faster recovery

### v1.1.0

- Transactions retry hints in `ErrConflict`

### v1.0.0

- First stable release

</details><!-- /readme-style:collapse -->

## Contributing

Issues and pull requests are welcome. Run `go test -race ./...` before sending one.

## License

Apache 2.0
