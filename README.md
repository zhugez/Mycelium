# Mycelium

**Mycelium** is a high-performance command-line tool (CLI) written in **Rust**, designed to automate the process of searching, downloading, and analyzing popular WordPress plugins from the official WordPress.org repository.

This project assists security researchers, developers, or data analysts in easily collecting large volumes of plugin samples for research or testing purposes.

## 🚀 Key Features

*   **High Speed**: Leverages the power of Rust and `tokio` for asynchronous processing, enabling extremely fast data downloading and handling.
*   **Parallel Downloading**: Supports multi-threading to download hundreds of plugins simultaneously without bottlenecks.
*   **Smart Filtering**: Allows filtering plugins based on popularity (Active Installs).
*   **Automation**: Automatically downloads ZIP files, extracts them, and organizes the directory structure neatly.
*   **Reporting**: Exports a list of plugins, versions, and download statuses to a CSV file (`plugins.csv`) for easy management.

## 📋 System Requirements

*   **Rust**: Latest stable version (to compile from source). Install via [rustup.rs](https://rustup.rs/).
*   **Internet Connection**: Required to connect to the WordPress.org API.

## 🛠️ Installation & Build

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/zhugez/Mycelium.git
    cd Mycelium
    ```

2.  **Build the project:**
    For best performance, build in `release` mode:
    ```bash
    cargo build --release
    ```
    After building, the executable will be located at:
    *   Windows: `target/release/mycelium.exe`
    *   Linux/macOS: `target/release/mycelium`

## 📖 Usage Guide

You can run it directly using `cargo run` or use the built binary.

### Basic Syntax

```bash
mycelium [OPTIONS]
```

### Options

| Option (Short/Long) | Default | Description |
| :--- | :--- | :--- |
| `-m`, `--min-active` | `10000` | Minimum number of active installs to download. |
| `-p`, `--pages` | `50` | Number of result pages to scan (WordPress API uses pagination). |
| `--per-page` | `100` | Number of plugins per page. |
| `-w`, `--workers` | `5` | Number of parallel download threads (workers). |
| `-o`, `--output-dir` | `wp_zips` | Directory to store downloaded `.zip` files. |
| `-e`, `--extract` | `false` | Enable this flag to automatically extract after downloading. |
| `--extract-dir` | `wp_extracted`| Directory for extracted plugins. |
| `-c`, `--csv-path` | `plugins.csv`| Path to the CSV report file. |
| `--list-only` | `false` | Only scan and list plugins, do not download. |

### Examples

1.  **Scan and download extremely popular plugins** (over 100,000 installs), saving to `hot_plugins` folder:
    ```bash
    cargo run -- -m 100000 -o hot_plugins
    ```

2.  **Download and immediately extract** the first 10 pages of plugins:
    ```bash
    cargo run -- --pages 10 --extract
    ```

3.  **List only (no download)** for preview:
    ```bash
    cargo run -- --list-only
    ```

## 🐍 Python Version

The project also includes an `index.py` file. This is a prototype version written in Python. It performs similar functions but may be slower than the Rust version. You can use it for logic reference or quick testing if you have Python and `uv`/`pip` installed.

To run (requires installing libraries in the script):
```bash
uv run python index.py
```

## 📄 Project Structure

```text
Mycelium/
├── src/
│   ├── main.rs       # Entry point, CLI argument handling
│   ├── api.rs        # Interaction with WordPress.org API
│   ├── fs_ops.rs     # File operations: download, ZIP extraction
│   ├── csv_ops.rs    # CSV Read/Write
│   ├── models.rs     # Data structs (Plugin, DownloadResult)
│   └── error.rs      # Centralized error management
├── index.py          # Python Version (Prototype)
├── Cargo.toml        # Rust dependency configuration
└── README.md         # Documentation
```

## 🤝 Contribution

All contributions are welcome! Please open Issues or Pull Requests on GitHub to improve the project.