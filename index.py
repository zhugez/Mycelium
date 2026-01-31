# wp_bulk_list_and_download.py

import json
import time
import html
import os
import zipfile
from pathlib import Path
import urllib.request
import urllib.parse
import csv
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Dict, Any, List, Tuple, Optional

BASE = "https://api.wordpress.org/plugins/info/1.2/"

HEADERS = {"User-Agent": "bulk-wp-plugin-research/1.0 (+contact: you@example.com)"}

# ---------- HTTP helpers ----------


def get_json(
    url: str, timeout: int = 25, retries: int = 4, backoff: float = 1.0
) -> Dict[str, Any]:
    req = urllib.request.Request(url, headers=HEADERS)
    last_err = None
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=timeout) as r:
                raw = r.read().decode("utf-8", errors="replace")
                return json.loads(raw)
        except Exception as e:
            last_err = e
            time.sleep(backoff * (2**attempt))
    raise last_err  # type: ignore[misc]


def download_file(
    url: str, dest: Path, timeout: int = 60, retries: int = 4, backoff: float = 1.0
) -> None:
    """
    Stream download to disk with retry/backoff.
    """
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_suffix(dest.suffix + ".part")

    last_err = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers=HEADERS)
            with (
                urllib.request.urlopen(req, timeout=timeout) as r,
                open(tmp, "wb") as f,
            ):
                while True:
                    chunk = r.read(1024 * 256)
                    if not chunk:
                        break
                    f.write(chunk)
            os.replace(tmp, dest)
            return
        except Exception as e:
            last_err = e
            # cleanup partial
            try:
                if tmp.exists():
                    tmp.unlink()
            except Exception:
                pass
            time.sleep(backoff * (2**attempt))

    raise last_err  # type: ignore[misc]


# ---------- Phase 1: list popular plugins ----------


def build_query_url(page: int, per_page: int) -> str:
    return (
        f"{BASE}?action=query_plugins"
        f"&request[browse]=popular"
        f"&request[per_page]={per_page}"
        f"&request[page]={page}"
    )


def fetch_page(page: int, per_page: int) -> Tuple[int, List[Dict[str, Any]]]:
    url = build_query_url(page, per_page)
    data = get_json(url)
    plugins = data.get("plugins", [])
    if not isinstance(plugins, list):
        plugins = []
    return page, plugins


def fetch_popular_parallel(
    min_active_installs: int = 10_000,
    pages: int = 50,
    per_page: int = 100,
    workers: int = 10,
    debug: bool = False,
) -> List[Dict[str, Any]]:
    by_slug: Dict[str, Dict[str, Any]] = {}

    with ThreadPoolExecutor(max_workers=workers) as ex:
        futures = [ex.submit(fetch_page, p, per_page) for p in range(1, pages + 1)]

        for fut in as_completed(futures):
            page, plugins = fut.result()
            if debug:
                print(f"done page {page} items {len(plugins)}")

            for p in plugins:
                slug = p.get("slug")
                if not slug:
                    continue

                try:
                    ai = int(p.get("active_installs") or 0)
                except Exception:
                    ai = 0

                if ai < min_active_installs:
                    continue

                name = html.unescape(p.get("name") or "")
                last_updated = p.get("last_updated")

                prev = by_slug.get(slug)
                if prev is None or ai > int(prev.get("active_installs", 0)):
                    by_slug[slug] = {
                        "slug": slug,
                        "name": name,
                        "active_installs": ai,
                        "last_updated": last_updated,
                        "page": page,
                    }

    out = sorted(
        by_slug.values(), key=lambda x: int(x["active_installs"]), reverse=True
    )
    return out


# ---------- Phase 2: resolve download link + download zip ----------


def get_download_link(slug: str) -> Tuple[Optional[str], Optional[str]]:
    """
    Uses plugin_information to get stable download link (zip) and version.
    Returns (download_link, version).
    """
    url = f"{BASE}?action=plugin_information&request[slug]={urllib.parse.quote(slug)}"
    info = get_json(url)
    dl = info.get("download_link")
    version = info.get("version")
    if isinstance(dl, str) and dl.startswith("http"):
        return dl, version
    return None, version


def download_one(slug: str, out_dir: Path) -> Tuple[str, bool, str, Optional[str]]:
    """
    Returns (slug, success, message, version).
    """
    dl, version = get_download_link(slug)
    if not dl:
        return slug, False, "no download_link", version

    dest = out_dir / f"{slug}.zip"
    if dest.exists() and dest.stat().st_size > 0:
        return slug, True, "already exists", version

    download_file(dl, dest)
    return slug, True, "downloaded", version


def save_plugins_to_csv(
    plugins: List[Dict[str, Any]],
    downloaded: List[Tuple[str, bool, str, Optional[str]]],
    csv_path: str = "plugins.csv",
) -> None:
    """
    Save plugin information to CSV file.
    """
    with open(csv_path, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["name", "version", "slug", "status"])

        downloaded_dict = {
            slug: (success, msg, version) for slug, success, msg, version in downloaded
        }

        for p in plugins:
            slug = p.get("slug", "")
            name = p.get("name", "")
            if slug in downloaded_dict:
                success, msg, version = downloaded_dict[slug]
                status = "downloaded" if success else f"failed: {msg}"
                writer.writerow([name, version or "", slug, status])
            else:
                writer.writerow([name, "", slug, "not downloaded"])

    print(f"Saved plugin info to {csv_path}")


def download_plugins(
    plugins: List[Dict[str, Any]],
    out_dir: str = "downloads",
    download_workers: int = 5,
    progress: bool = True,
) -> None:
    out_path = Path(out_dir)
    out_path.mkdir(parents=True, exist_ok=True)

    slugs = [p["slug"] for p in plugins if p.get("slug")]

    ok = 0
    fail = 0
    results: List[Tuple[str, bool, str, Optional[str]]] = []

    with ThreadPoolExecutor(max_workers=download_workers) as ex:
        futures = {ex.submit(download_one, slug, out_path): slug for slug in slugs}

        for fut in as_completed(futures):
            slug, success, msg, version = fut.result()
            results.append((slug, success, msg, version))
            if success:
                ok += 1
            else:
                fail += 1
            if progress:
                print(f"[{'OK' if success else 'FAIL'}] {slug}: {msg}")

    print(f"\nDownload done. OK={ok} FAIL={fail}  -> folder: {out_path.resolve()}")

    save_plugins_to_csv(plugins, results, "plugins.csv")

    return results


def extract_all_zips(
    zip_dir: str = "wp_zips",
    extract_dir: str = "wp_extracted",
    workers: int = 5,
    progress: bool = True,
) -> None:
    """
    Giải nén toàn bộ file zip trong thư mục zip_dir vào extract_dir.
    """
    zip_path = Path(zip_dir)
    extract_path = Path(extract_dir)
    extract_path.mkdir(parents=True, exist_ok=True)

    zip_files = list(zip_path.glob("*.zip"))
    if not zip_files:
        print(f"Không tìm thấy file zip nào trong {zip_path}")
        return

    print(f"\n--- Giải nén {len(zip_files)} plugins ---")

    def extract_one(zip_file: Path) -> Tuple[str, bool, str]:
        slug = zip_file.stem
        dest = extract_path / slug
        try:
            if dest.exists():
                return slug, True, "đã tồn tại"
            with zipfile.ZipFile(zip_file, "r") as zf:
                zf.extractall(extract_path)
            return slug, True, "giải nén thành công"
        except zipfile.BadZipFile:
            return slug, False, "file zip bị lỗi"
        except Exception as e:
            return slug, False, str(e)

    ok = 0
    fail = 0

    with ThreadPoolExecutor(max_workers=workers) as ex:
        futures = {ex.submit(extract_one, zf): zf for zf in zip_files}

        for fut in as_completed(futures):
            slug, success, msg = fut.result()
            if success:
                ok += 1
            else:
                fail += 1
            if progress:
                print(f"[{'OK' if success else 'FAIL'}] {slug}: {msg}")

    print(f"\nGiải nén hoàn tất. OK={ok} FAIL={fail} -> folder: {extract_path.resolve()}")


# ---------- main ----------

if __name__ == "__main__":
    # LIST settings
    MIN_ACTIVE = 10_000
    PAGES = 50
    PER_PAGE = 100
    LIST_WORKERS = 10

    # DOWNLOAD settings
    DOWNLOAD_DIR = "wp_zips"
    DOWNLOAD_WORKERS = 5  # increase to 10 if you really want, but 5 is nicer

    # EXTRACT settings
    EXTRACT_AFTER_DOWNLOAD = True  # Đặt True để giải nén sau khi download
    EXTRACT_DIR = "wp_extracted"
    EXTRACT_WORKERS = 5

    plugins = fetch_popular_parallel(
        min_active_installs=MIN_ACTIVE,
        pages=PAGES,
        per_page=PER_PAGE,
        workers=LIST_WORKERS,
        debug=False,
    )

    print("Unique plugins:", len(plugins))
    for p in plugins[:30]:
        print(p["active_installs"], p["slug"], "-", p["name"], f"(page {p['page']})")

    print("\n--- Downloading zips (stable) ---")
    download_plugins(
        plugins=plugins,
        out_dir=DOWNLOAD_DIR,
        download_workers=DOWNLOAD_WORKERS,
        progress=True,
    )

    # Giải nén nếu option được bật
    if EXTRACT_AFTER_DOWNLOAD:
        extract_all_zips(
            zip_dir=DOWNLOAD_DIR,
            extract_dir=EXTRACT_DIR,
            workers=EXTRACT_WORKERS,
            progress=True,
        )
