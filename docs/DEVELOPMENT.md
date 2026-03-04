# Розробка Llama-RS

Покроковий гайд для Rust-розробника: збірка, тести, бенчмарки, git.

## Передумови

- **Rust**: встановлений через [rustup](https://rustup.rs).
- **64-bit Windows target**:
  ```bash
  rustup default stable-x86_64-pc-windows-msvc
  ```
  Для збірки MSVC-таргету потрібні **Build Tools for Visual Studio** (компонент "Desktop development with C++") — інакше з’явиться помилка `link.exe not found`. Альтернатива — GNU-таргет: `rustup default stable-x86_64-pc-windows-gnu` (потрібен MinGW-w64).
- **llama.cpp**: клон або копія master гілки у сусідній папці (наприклад `../llama.cpp-master`). Точний шлях налаштовується в `build.rs` або через змінну середовища.

## Збірка

```bash
cd llama-rs-project
cargo build
```

Реліз (оптимізований 64-bit exe):

```bash
cargo build --release
```

Артефакт: `target\release\llama_rs.exe`.

## Тести

```bash
cargo test
```

- Unit-тести в `src/lib.rs`.
- Інтеграційні тести в `tests/`.

## Бенчмарки (ультрашвидкість)

```bash
cargo bench
```

Критерії швидкості описані в `benches/`. Після інтеграції з llama.cpp тут можна додати вимірювання токенів/сек та часу першого токена.

## Git та перший коміт

1. Ініціалізація (якщо ще не зроблено):
   ```bash
   cd llama-rs-project
   git init
   ```

2. Додати файли та перший коміт:
   ```bash
   git add .
   git commit -m "hello llama rust"
   ```

3. Віддалений репозиторій та push (з gittoken):
   - Створити репо на GitHub/GitLab тощо.
   - Додати remote:
     ```bash
     git remote add origin https://github.com/YOUR_USER/llama-rs-project.git
     ```
   - При авторизації використати Personal Access Token (gittoken) замість пароля:
     ```bash
     git push -u origin main
     ```
   - Якщо гілка називається `master`: `git push -u origin master`.

## Корисні команди

| Дія           | Команда                |
|---------------|------------------------|
| Збірка        | `cargo build --release`|
| Тести         | `cargo test`           |
| Бенчмарки     | `cargo bench`          |
| Перевірка     | `cargo check`          |
| Лінтер        | `cargo clippy`         |
| Формат        | `cargo fmt`            |

Документація архітектури та плану — у [PLAN.md](PLAN.md), [ARCHITECTURE.md](ARCHITECTURE.md), [CONCEPT.md](CONCEPT.md).
