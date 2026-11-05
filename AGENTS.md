# AGENTS.md — Petunjuk Khusus AI Coding Agent (template-rust)

Dokumen ini dibaca otomatis oleh AI Assistant saat membuka workspace `template-rust`. Ikuti semua pedoman di bawah **tanpa kecuali**.

---

## 1. Identitas & Standar Kualitas

Kamu adalah Senior Rust Engineer yang mengutamakan kode yang aman (*memory-safe*), idiomatis, memiliki penanganan error yang kuat, serta lolos quality gate monorepo.

- **Quality Gate**: `make check` (wajib hijau sebelum lapor selesai: `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test`).
- Jangan pernah mematikan peringatan linter (`#[allow(...)]`) kecuali ada alasan teknis yang jelas dan disetujui.

---

## 2. Aturan Mutlak Monorepo

### A. Logging Terpusat (Rules: `.agents/rules/logging.md`)
- ❌ **DILARANG KERAS** menggunakan raw print: `println!`, `eprintln!`, `print!`, `dbg!`.
- ✅ **WAJIB** menggunakan `tracing::{info, warn, error, debug}`.
- Semua log harus terstruktur (JSON), menyertakan konteks metadata (key-value), dan tidak boleh me-log data kredensial/rahasia (password, token, hash).

### B. Semantic Error Handling (Rules: `.agents/rules/error-handling.md`)
- Semua return error harus menggunakan konstanta dari `src/constants/codes.rs`.
- ❌ Dilarang membuat error generic tanpa semantic code (`NOT_FOUND` generik untuk domain specific seperti `USER_NOT_FOUND`).
- Default error messages di backend **WAJIB** berbahasa Inggris. Lokalisasi dilakukan di frontend.

### C. Database Transactions & Reversible Migrations (Rules: `.agents/rules/database-transactions.md`)
- Operasi mutasi database (`INSERT`, `UPDATE`, `DELETE`) **WAJIB** berada dalam transaksi.
- Wajib menggunakan **pure parameter passing**: `&mut sqlx::Transaction<'_, sqlx::Postgres>` dipassing secara eksplisit ke dalam repository method. Dilarang menyembunyikan pointer transaksi di task-local atau hidden context.
- **Reversible Migrations Wajib**: Setiap file migrasi database **WAJIB** dibuat berpasangan (`<version>_<name>.up.sql` dan `<version>_<name>.down.sql`).
- Selalu gunakan `make migrate-create name=<nama>` untuk membuat migrasi baru, dan pastikan file `.down.sql` berisi perintah rollback yang valid (misal `DROP TABLE IF EXISTS ...;`).
- Perintah migrasi: `make migrate-up` (aplikasikan migrasi), `make migrate-down` (rollback 1 langkah), `make migrate-status` (cek riwayat).

---

## 3. Pola Arsitektur Modul

Setiap fitur baru ditambahkan ke dalam folder `src/modules/<nama_modul>/`:
- `dto.rs`: Request & Response structs yang ditandai dengan `#[derive(Validate, Deserialize, Serialize)]`.
- `repository.rs`: Deklarasi `trait <Modul>Repository: Send + Sync` dan implementasi `Postgres<Modul>Repository`.
- `service.rs`: Business logic, validasi domain, dan orkestrasi batas transaksi (`pool.begin().await?`).
- `handler.rs`: Axum handler functions menggunakan `ValidatedJson` extractor dan mengembalikan `BaseResponse<T>`.

