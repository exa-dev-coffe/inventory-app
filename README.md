# ☕ Coffe Inventory & Recipe (BOM) Microservice (`inventory-app`)

Microservice berbasis **Rust (Axum + Tonic gRPC + SQLx + RabbitMQ)** untuk ekosistem microservices **Coffe**. Bertanggung jawab atas pengelolaan bahan baku mentah (*Raw Materials*), resep menu (*Bill of Materials* / BOM), pemotongan stok otomatis (*atomic stock deduction*), dan audit log pergerakan stok.

---

## 🚀 Fitur Utama

- **Master Bahan Baku (`/api/v1/ingredients`)**:
  - Pelacakan stok biji kopi (gram), susu/sirup (ml), dan paper cup/kemasan (pcs).
  - Peringatan stok menipis (*low-stock threshold warning*).
  - Perhitungan estimasi biaya bahan (*Cost of Goods Sold* / COGS).
- **Resep Menu / Bill of Materials (`/api/v1/recipes/:menu_id`)**:
  - Relasi komposisi takaran bahan baku per 1 porsi menu (`menu_id` dari `master-data-app`).
- **Pencatatan Masuk & Opname (`/api/v1/stock/adjust` & `/api/v1/stock/logs`)**:
  - Pencatatan barang masuk (*IN_PURCHASE*), barang rusak/tumpah (*OUT_WASTE*), dan penyesuaian opname (*ADJUSTMENT_OPNAME*).
  - Audit trail pergerakan stok lengkap dengan riwayat saldo akhir.
- **High-Performance gRPC Server (Tonic)**:
  - Port `:9086` melayani `CheckStockAvailability`, `DeductStock`, dan `RollbackStock` secara atomic bebas *race condition*.
- **HTTP REST API (Axum 0.7)**:
  - Port `:8086` untuk manajemen CRUD oleh dashboard admin atau kasir.

---

## ⚡ Pemetaan Port

| Protokol | Port | Keterangan |
| :--- | :--- | :--- |
| **HTTP REST** | `:8086` | Health check (`/health`), CRUD ingredients, recipes, & stock adjustments |
| **gRPC** | `:9086` | Inter-service East-West RPC dengan `transaction-app` |

---

## 📄 Protobuf Contract (`proto/inventory/inventory.proto`)

```protobuf
syntax = "proto3";

package inventory;

option go_package = "coffe/proto/inventory;inventorypb";

service InventoryService {
  rpc CheckStockAvailability (CheckStockRequest) returns (CheckStockResponse);
  rpc DeductStock (DeductStockRequest) returns (DeductStockResponse);
  rpc RollbackStock (RollbackStockRequest) returns (RollbackStockResponse);
}
```

---

## 🛠️ Quickstart

### 1. Jalankan Unit Test
```bash
cargo test --test inventory_test
```

### 2. Jalankan Migrasi Database
```bash
cargo run --bin migrate
```

### 3. Jalankan Server (REST + gRPC)
```bash
cargo run
```
