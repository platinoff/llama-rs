# План проєкту Llama-RS

## Мета

Створити архітектуру Rust-проєкту для роботи з **llama.cpp** (master), з максимальною часткою коду на Rust для ультрашвидкої та безпечної роботи, збірка — 64-бітний `.exe`.

---

## 1. Вимоги

| Вимога | Рішення |
|--------|---------|
| Джерело llama | Папка `../llama.cpp-master` (або `llama.cpp-master/llama.cpp-master` при вкладеній структурі) |
| Мова | Максимум Rust, мінімум FFI до C/C++ |
| Безпека | Safe Rust API, небезпечний код лише в тонкому шарі біндингів |
| Швидкість | Zero-copy де можливо, батчінг, мінімум алокацій на шляху інференсу |
| Результат збірки | `target/release/llama_rs.exe` (x86_64-pc-windows-msvc) |
| Ліцензія | MIT |
| СКВ | Git-friendly структура, один репозиторій |

---

## 2. Структура проєкту (Git-friendly)

```
llama-rs-project/
├── .gitignore
├── Cargo.toml              # workspace або пакет lib+bin
├── LICENSE                  # MIT
├── README.md
├── build.rs                 # збірка llama.cpp та/або біндингів
├── src/
│   ├── lib.rs               # публічне API (safe Rust)
│   ├── main.rs              # CLI (exe)
│   ├── ffi/                 # низькорівневі біндинги до llama.cpp (optional mod)
│   └── safe/                # high-level safe обгортки (optional mod)
├── docs/
│   ├── PLAN.md              # цей план
│   ├── ARCHITECTURE.md      # архітектура та діаграми
│   ├── CONCEPT.md           # концепція та рішення
│   └── DEVELOPMENT.md       # гайд для Rust-розробника
├── tests/                   # інтеграційні тести
├── benches/                 # критерії швидкості (ultra-speed)
└── llama.cpp-master/        # опційно: git submodule на master
```

- У репозиторії не зберігати бінарники та артефакти збірки (`target/`, `llama.cpp/build/`).
- Можливий **git submodule** на офіційний `llama.cpp` master замість копії в `../llama.cpp-master`.

---

## 3. Етапи реалізації

### Етап 1 — Hello Llama Rust (перший коміт)

- [x] Ініціалізація Cargo-проєкту (lib + bin).
- [x] `docs/PLAN.md` — план.
- [x] `README.md` — опис проєкту як ультрашвидкого.
- [x] MIT `LICENSE`, `.gitignore`.
- [x] Перший коміт: "hello llama rust", перший push (за бажанням — gittoken/remote).

### Етап 2 — Інтеграція з master-папкою

- [ ] Визначити точний шлях до llama.cpp (змінна середовища або `build.rs`).
- [ ] Збірка libllama (static) з `../llama.cpp-master` через `cmake` або `cc` у `build.rs`.
- [ ] Генерація біндингів (e.g. `bindgen`) до `llama.h` / C API.
- [ ] Тонкий unsafe-шар у `src/ffi` та safe-обгортки в `src/safe` або в `lib.rs`.

### Етап 3 — Документація та архітектура

- [ ] `docs/ARCHITECTURE.md` — модулі, залежності, потік даних.
- [ ] `docs/CONCEPT.md` — концепція (Rust-first, безпека, швидкість).
- [ ] `docs/DEVELOPMENT.md` — як збирати, тестувати, бенчити (rustc, cargo).

### Етап 4 — Тести та ультрашвидкість

- [ ] Unit-тести для safe API.
- [ ] Інтеграційні тести (з мінімальною моделлю або mock).
- [ ] `benches/` — benchmark’и (наприклад, токени/сек, час першого токена).
- [ ] Документування результатів у `docs/` та перевірка збірки 64-bit exe.

---

## 4. Інструменти

- **rustc** — компілятор (через `cargo`).
- **cargo** — збірка, тести, бенчі.
- **git** — версіонування; при push — **gittoken** (Personal Access Token або credential helper).

---

## 5. Цільова платформа

- **OS:** Windows (згідно з шляхами S:\rust\...).
- **Target:** `x86_64-pc-windows-msvc` для отримання 64-бітного `.exe`.

Перевірка:

```bash
rustup default stable-x86_64-pc-windows-msvc
cargo build --release
# Результат: target/release/llama_rs.exe (або ім’я пакету з Cargo.toml)
```

---

## 6. Перший коміт

- Повідомлення: **hello llama rust**
- Вміст: план у `docs/`, README, LICENSE, `.gitignore`, мінімальний `src/lib.rs` та `src/main.rs`, що виводять привітання. Після цього — перший `git push` (з gittoken при потребі).

Цей план є живим документом і може оновлюватися в `docs/` по мірі розвитку проєкту.
