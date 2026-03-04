# Архітектура Llama-RS

## Огляд

Llama-RS будується як **Rust-first** шар над C API llama.cpp. Архітектура розділена на рівні з мінімальною кількістю небезпечного коду.

## Рівні

```
┌─────────────────────────────────────────────────────────┐
│  CLI / Application (main.rs, safe Rust)                 │
├─────────────────────────────────────────────────────────┤
│  Public API (lib.rs, safe Rust)                          │
│  - Model loading, context, sampling, batching           │
├─────────────────────────────────────────────────────────┤
│  Safe wrappers (optional: src/safe/)                     │
│  - RAII, Result, idiomatic types                        │
├─────────────────────────────────────────────────────────┤
│  FFI layer (optional: src/ffi/)                         │
│  - bindgen-generated + thin unsafe wrappers             │
├─────────────────────────────────────────────────────────┤
│  llama.cpp (C/C++) — libllama static                     │
│  - Built from ../llama.cpp-master                        │
└─────────────────────────────────────────────────────────┘
```

## Модулі (майбутня структура)

| Модуль   | Призначення |
|----------|-------------|
| `lib.rs` | Публічне API, реекспорт, `hello_llama_rust` та майбутні функції |
| `ffi`    | Низькорівневі виклики C API (unsafe), генерація через bindgen |
| `safe`   | Safe обгортки (Context, Model, Sampler тощо) |

## Потік даних (майбутній інференс)

1. **Завантаження моделі** — шлях до GGUF → FFI `llama_load_model_from_file` → safe `Model`.
2. **Контекст** — `Model` + params → `llama_new_context_with_model` → safe `Context`.
3. **Декодування** — Rust формує `llama_batch`, викликає `llama_decode` → логіти повертаються в Rust.
4. **Семплінг** — логіти → sampler API → наступний token; цикл у Rust.

## Залежності збірки

- **build.rs**: визначає шлях до llama.cpp (env або відносний), викликає cmake/cc для збірки libllama, при потребі — bindgen.
- **Cargo.toml**: залежності `cmake`, `bindgen` (якщо використовуємо власні біндинги); можливий варіант з крейтом `llama-cpp-2` та path до локальної збірки.

## Цільова платформа

- **x86_64-pc-windows-msvc** — релізний артефакт: один 64-бітний `llama_rs.exe`.

Документ оновлюватиметься з появою нових модулів і інтеграції з llama.cpp.
