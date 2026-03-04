# Концепція Llama-RS

## Ідея

Максимум коду на **Rust** для швидкості та безпеки; мінімальний стабільний FFI до llama.cpp. Кінцевий продукт — один 64-бітний exe, зібраний cargo.

## Принципи

1. **Safe by default** — небезпечний код лише в ізольованому FFI-шарі; решта — safe Rust.
2. **Zero-cost abstractions** — абстракції не додають накладних витрат у release.
3. **Ultra-speed** — мінімум алокацій на шляху інференсу, батчінг, можливість zero-copy де це можливо.
4. **Git-friendly** — чиста структура, submodule або зовнішній шлях до llama.cpp, без великих бінарників у репо.
5. **MIT** — вільна ліцензія для використання та розповсюдження.

## Джерело даних

- Робоча версія **llama.cpp** береться з папки **master** (наприклад `S:\rust\llama-rs\llama.cpp-master` або вкладена `llama.cpp-master/llama.cpp-master`).
- Збірка libllama виконується під час `cargo build` (build.rs) або вручну з подальнім посиланням.

## Інструменти розробки

- **rustc** (через cargo) — компіляція.
- **cargo** — build, test, bench.
- **git** — версіонування; при push — **gittoken** (token або credential helper).

Цей документ описує загальну концепцію; деталі реалізації — у [PLAN.md](PLAN.md) та [ARCHITECTURE.md](ARCHITECTURE.md).
