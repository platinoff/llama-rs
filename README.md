# llama.rs · Llama in Rust

**llama.rs** is a **Rust-native** implementation of Llama inference: API, orchestration, and control flow are written in Rust. The compute backend is [llama.cpp](https://github.com/ggml-org/llama.cpp) (via the `llama-cpp-2` crate), but **this codebase is 100% Rust** — no C/C++ in the repo.

## Why Rust all the way?

- **llama.rs, not llama.cpp** — All code you see here is **Rust**. The inference loop (tokenize → decode → sample → accept) lives in `src/safe/`; only the heavy math runs in the linked llama.cpp backend.
- **Zero-cost abstractions** — Thin wrappers (Backend, Model, Context, generate) add no extra allocation on the hot path.
- **64-bit native** — Release builds produce a single `llama_rs.exe` (x86_64-pc-windows-msvc).
- **Safe by default** — Idiomatic `Result` and `Error`; no `unsafe` in this repository.

## Quick start

### Easy install (Windows)

Якщо вже встановлені **Rust** ([rustup](https://rustup.rs)) та **Visual Studio Build Tools** з workload’ами **"Desktop development with C++"** і **"C++ Clang tools for Windows"**, достатньо одного скрипта:

```powershell
git clone https://github.com/platinoff/llama-rs.git
cd llama-rs
.\install.ps1
```

**Що робить скрипт:** шукає `libclang.dll` (потрібен для збірки бекенду llama.cpp), виставляє змінну `LIBCLANG_PATH`, запускає середовище MSVC (`VsDevCmd`) і виконує `cargo build --release`. Після успішної збірки ви отримаєте виконуваний файл `.\target\release\llama_rs.exe`.

**Запуск:**

```powershell
.\target\release\llama_rs.exe
# або з моделлю GGUF:
.\target\release\llama_rs.exe path\to\model.gguf "Your prompt"
```

Якщо скрипт не знаходить Clang або VsDevCmd — перевірте [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#prerequisites). Нижче — покрокова збірка вручну, якщо хочете розуміти кожен крок.

---

### Покрокова збірка (ручна)

**1. Клонування репозиторію**

```bash
git clone https://github.com/platinoff/llama-rs.git
cd llama-rs
```

(Якщо клонували в іншу папку — далі підставляйте її замість `llama-rs`.)

**2. Що потрібно один раз (prerequisites)**

- **Rust** — [rustup](https://rustup.rs), далі наприклад `rustup default stable-x86_64-pc-windows-msvc` для 64-бітної Windows. Потрібен для компіляції коду llama.rs і виклику збірки через `cargo`.
- **Visual Studio Build Tools** — workload **"Desktop development with C++"**: дає лінкер `link.exe` і середовище MSVC, без нього Rust не збере програму під Windows.
- **Clang** — у VS Installer виберіть **"C++ Clang tools for Windows"**. Збірка крейту `llama-cpp-2` використовує `libclang.dll` для парсингу C/C++ заголовків; без цього кроку збірка впаде з помилкою про `LIBCLANG_PATH`.  
  Детально: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#prerequisites).

**3. Збірка**

На **Windows** потрібні середовище MSVC і змінна `LIBCLANG_PATH`. У PowerShell (шлях до Clang можна змінити під свою версію VS):

```powershell
cd llama-rs
$env:LIBCLANG_PATH = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\Llvm\x64\bin"
cmd /c "`"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat`" -arch=amd64 && cd /d %CD% && cargo build --release"
```

**Що це робить:** `LIBCLANG_PATH` вказує, де взяти `libclang.dll`. `VsDevCmd.bat` налаштовує PATH і змінні для лінкера MSVC. `cargo build --release` компілює проект у режимі release; результат — `target\release\llama_rs.exe`.

Або відкрийте **"x64 Native Tools Command Prompt for VS"** з меню Пуск і виконайте:

```cmd
set LIBCLANG_PATH=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\Llvm\x64\bin
cd path\to\llama-rs
cargo build --release
```

**4. Запуск**

```bash
.\target\release\llama_rs.exe
# з моделлю GGUF:
.\target\release\llama_rs.exe path\to\model.gguf "Your prompt"
```

Перший запуск без аргументів виводить привітання; з шляхом до моделі — завантажує її і генерує текст. Детальніше: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Requirements

- **Rust** 1.70+ (e.g. `rustup default stable-x86_64-pc-windows-msvc` on Windows).
- **Windows (MSVC):** Build Tools with "Desktop development with C++" and "C++ Clang tools for Windows"; set `LIBCLANG_PATH` to the Clang `bin` folder when building.
- **Backend:** llama.cpp is built automatically by the `llama-cpp-2` dependency (no separate clone). See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for details.

## Project layout

| Path         | Description |
|-------------|-------------|
| `install.ps1` | Easy install script (Windows): sets LIBCLANG_PATH, runs MSVC + cargo build |
| `src/lib.rs`  | Public API (Rust) |
| `src/main.rs` | CLI (64-bit exe) |
| `src/safe/`   | Backend, Model, Context, generate, generate_stream, embed |
| `src/error.rs`| Error and Result types |
| `src/metrics.rs` | InferenceMetrics (optional feature) |
| `docs/`       | Plan, architecture, guides |
| `tests/`      | Integration tests |
| `benches/`    | Benchmarks |

## Documentation

- [docs/PLAN.md](docs/PLAN.md) — Project plan and phases.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — Architecture and module layout.
- [docs/CONCEPT.md](docs/CONCEPT.md) — Design concepts (Rust-first, safety, speed).
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) — Build, test, benchmark (rustc, cargo, git).
- [docs/NEXT_STEPS.md](docs/NEXT_STEPS.md) — Prioritized roadmap.
- [docs/BENCHMARKS.md](docs/BENCHMARKS.md) — Benchmarks and metrics.
- [docs/SIZING.md](docs/SIZING.md) — n_ctx / n_batch and memory.
- [docs/GITHUB_SETUP.md](docs/GITHUB_SETUP.md) — GitHub repo and push.

## Support the developer

If you find llama.rs useful and want to support its development, you can send **Solana (SOL)** to:

```
GcdgNtdE8NEk3z9sQ5jXv2tqguZjSYqPqNAtjsjPNJx8
```

Thank you.

## License

MIT — see [LICENSE](LICENSE).

## Contributing

See `docs/DEVELOPMENT.md` for build and test instructions. Use `cargo test` and `cargo bench` to verify correctness and ultra-speed.
