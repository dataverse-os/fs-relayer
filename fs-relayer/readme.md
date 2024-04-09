# File system Relayer

## [API Documentation](./api.md)

## Running
After building the project, you can run it using the following command:

```shell
cargo run
```

## Configuration


```rust
pub struct Config {
    data_path: Option<String>,
    pub kubo_path: String,
    pub networks: Vec<Network>,

    pub queue_dsn: String,
    pub queue_pool: u32,
    pub queue_worker: u32,

    pub cache_size: usize,
    pub index_models: IndexModels,
    pub ceramic: String,

    pub pgsql_dsn: Option<String>,
    pub iroh: Option<IrohConfig>,
}
```

`pgsql_dsn` and `iroh` are optional, you should provide one of them.

## Building

To build the project, run:

```shell
cargo build
```

## Build with docker

```shell
docker build -t fs-relayer .
```