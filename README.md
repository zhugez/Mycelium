# Mycelium

**Mycelium** là một công cụ dòng lệnh (CLI) hiệu suất cao được viết bằng **Rust**, được thiết kế để tự động hóa việc tìm kiếm, tải xuống và phân tích các plugin WordPress phổ biến từ kho lưu trữ chính thức WordPress.org.

Dự án này giúp các nhà nghiên cứu bảo mật, lập trình viên hoặc nhà phân tích dữ liệu dễ dàng thu thập mẫu plugin số lượng lớn để nghiên cứu hoặc kiểm tra.

## 🚀 Tính năng nổi bật

*   **Tốc độ cao**: Tận dụng sức mạnh của Rust và `tokio` để xử lý không đồng bộ, cho phép tải xuống và xử lý dữ liệu cực nhanh.
*   **Tải xuống song song**: Hỗ trợ đa luồng (multi-threading) để tải xuống hàng trăm plugin cùng lúc mà không bị tắc nghẽn.
*   **Bộ lọc thông minh**: Cho phép lọc plugin dựa trên mức độ phổ biến (số lượng cài đặt kích hoạt - Active Installs).
*   **Tự động hóa**: Tự động tải file ZIP, giải nén (extract) và tổ chức thư mục gọn gàng.
*   **Xuất báo cáo**: Lưu danh sách plugin, phiên bản và trạng thái tải xuống vào file CSV (`plugins.csv`) để dễ dàng quản lý.

## 📋 Yêu cầu hệ thống

*   **Rust**: Phiên bản ổn định mới nhất (để biên dịch từ mã nguồn). Cài đặt tại [rustup.rs](https://rustup.rs/).
*   **Kết nối Internet**: Để kết nối với API của WordPress.org.

## 🛠️ Cài đặt & Biên dịch (Build)

1.  **Clone repository:**
    ```bash
    git clone https://github.com/zhugez/Mycelium.git
    cd Mycelium
    ```

2.  **Biên dịch dự án:**
    Để có hiệu suất tốt nhất, hãy build ở chế độ `release`:
    ```bash
    cargo build --release
    ```
    Sau khi build xong, file thực thi sẽ nằm tại:
    *   Windows: `target/release/mycelium.exe`
    *   Linux/macOS: `target/release/mycelium`

## 📖 Hướng dẫn sử dụng

Bạn có thể chạy trực tiếp bằng `cargo run` hoặc sử dụng file binary đã build.

### Cú pháp cơ bản

```bash
mycelium [OPTIONS]
```

### Các tùy chọn (Options)

| Tùy chọn (Short/Long) | Mặc định | Mô tả |
| :--- | :--- | :--- |
| `-m`, `--min-active` | `10000` | Số lượng cài đặt kích hoạt tối thiểu để tải về. |
| `-p`, `--pages` | `50` | Số lượng trang kết quả cần quét (API WordPress phân trang). |
| `--per-page` | `100` | Số lượng plugin trên mỗi trang. |
| `-w`, `--workers` | `5` | Số lượng luồng (workers) tải xuống song song. |
| `-o`, `--output-dir` | `wp_zips` | Thư mục lưu trữ các file `.zip` tải về. |
| `-e`, `--extract` | `false` | Bật cờ này để tự động giải nén sau khi tải xong. |
| `--extract-dir` | `wp_extracted`| Thư mục chứa các plugin đã được giải nén. |
| `-c`, `--csv-path` | `plugins.csv`| Đường dẫn file CSV báo cáo kết quả. |
| `--list-only` | `false` | Chỉ quét và liệt kê danh sách, không tải xuống. |

### Ví dụ minh họa

1.  **Quét và tải các plugin cực kỳ phổ biến** (trên 100,000 cài đặt), lưu vào thư mục `hot_plugins`:
    ```bash
    cargo run -- -m 100000 -o hot_plugins
    ```

2.  **Tải và giải nén ngay lập tức** 10 trang plugin đầu tiên:
    ```bash
    cargo run -- --pages 10 --extract
    ```

3.  **Chỉ lấy danh sách (không tải)** để xem trước:
    ```bash
    cargo run -- --list-only
    ```

## 🐍 Phiên bản Python

Dự án cũng đi kèm một file `index.py`. Đây là phiên bản prototype (nguyên mẫu) được viết bằng Python. Nó có chức năng tương tự nhưng có thể chậm hơn phiên bản Rust. Bạn có thể dùng nó để tham khảo logic hoặc chạy thử nghiệm nhanh nếu đã cài sẵn Python và `uv`/`pip`.

Cách chạy (yêu cầu cài các thư viện trong script):
```bash
uv run python index.py
```

## 📄 Cấu trúc dự án

```text
Mycelium/
├── src/
│   ├── main.rs       # Entry point, xử lý tham số dòng lệnh
│   ├── api.rs        # Tương tác với WordPress.org API
│   ├── fs_ops.rs     # Xử lý file: tải xuống, giải nén ZIP
│   ├── csv_ops.rs    # Đọc/Ghi file CSV
│   ├── models.rs     # Các struct dữ liệu (Plugin, DownloadResult)
│   └── error.rs      # Quản lý lỗi tập trung
├── index.py          # Phiên bản Python (Prototype)
├── Cargo.toml        # Cấu hình dependency Rust
└── README.md         # Tài liệu hướng dẫn
```

## 🤝 Đóng góp

Mọi đóng góp đều được hoan nghênh! Hãy mở Issues hoặc Pull Requests trên GitHub để cải thiện dự án.
