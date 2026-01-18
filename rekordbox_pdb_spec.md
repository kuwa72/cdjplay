# Rekordbox export.pdb Technical Specification (Refined)

This document summarizes the internal structure of the `export.pdb` database, cross-referenced with external analysis from [DJ Link Ecosystem Analysis](https://djl-analysis.deepsymmetry.org/rekordbox-export-analysis/exports.html).

## 1. File Header

| Offset | Size | Type | Description |
| :--- | :--- | :--- | :--- |
| 0x04 | 4 | UInt32 LE | **len_page**: Page size (default 4096). |
| 0x08 | 4 | UInt32 LE | **num_tables**: Number of tables. |
| 0x1c | 16*N | Array | **Table Pointers** (one per table). |

### Table Pointer Structure (16 bytes)
- `+0x00`: **Type** (Known: 0=Tracks, 1=Genres, 2=Artists, 3=Albums, 8=Playlist Entries).
- `+0x04`: **empty_candidate** (Internal use).
- `+0x08`: **first_page_idx** (Page Index, 0-indexed).
- `+0x0c`: **row_size** (Not used for metadata tables, use page navigation).

---

## 2. Page Structure

### Page Header
| Offset | Size | Type | Description |
| :--- | :--- | :--- | :--- |
| 0x0C | 4 | UInt32 LE | **next_page_index**. |
| 0x18 | 2 | UInt16 LE | **Row counts** (Packed fields). |
| 0x1B | 1 | Byte | **page_flags**. Data page if `pf & 0x40 == 0`. |

> [!IMPORTANT]
> ### Row Counting and Offsets
The 2-byte value at offset `0x18` in the page header is **masked** to represent both the row count and the number of offsets in the footer:
- `num_rows = value & 0x07FF` (Bottom 11 bits)
- `num_row_offsets = value >> 11` (Top 13 bits)

> [!IMPORTANT]
> Some pages have `num_rows` (total logical records) different from `num_row_offsets` (physical pointers in footer). Always use `num_row_offsets` to partition the page data.

### Page Footer
- **Offset Array**: `(PageSize) - (num_row_offsets * 2)` to `PageSize`.
- Each UInt16 LE points to the row start relative to the page beginning.

---

## 3. Metadata Table Rows

### Track Rows (Type 0 / TID 557)
- `0x08 - 0x0B`: **sample_rate**
- `0x38 - 0x3B`: **tempo (BPM * 100)** (UInt32 LE)
- `0x3C - 0x3F`: **genre_id**
- `0x40 - 0x43`: **album_id**
- `0x44 - 0x47`: **artist_id**
- `0x48 - 0x4B`: **id** (Track ID used in playlist entries)

### Artist Rows (Type 2 / TID 574)
- `0x00 - 0x01`: **subtype** (`0x60` near, `0x64` far)
- `0x04 - 0x07`: **id** (Artist ID)
- `0x09` (near) or `0x0A` (far): **ofs_name**
- **Name Address**: `RowStart + ofs_name`.

### Album Rows (Type 3 / TID 577)
- `0x00 - 0x01`: **subtype** (`0x80` near, `0x84` far)
- `0x08 - 0x0B`: **artist_id**
- `0x0C - 0x0F`: **id** (Album ID)
- `0x15` (near) or `0x16` (far): **ofs_name**

---

## 4. Strings (DeviceSQL)
- **Flag Byte**:
    - `0x90`: UTF-16LE.
    - `0x40`: ASCII.
    - If Bit 0 set: **Short ASCII**. Length = `flag >> 1`.
