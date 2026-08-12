# Mini-Launchpad

Учебный Solana/Anchor проект (модуль 7, см. `TODO.md` — задание на русском): ценовой
оракул + фабрика SPL-токенов с динамической комиссией, минимальный backend на axum
и Preact-фронтенд с подключением кошелька Phantom.

## Структура репозитория

Cargo workspace (`Cargo.toml`, `members = ["backend", "programs/*", "scripts/*"]`):

| Путь                        | Что это                                                                 |
|------------------------------|--------------------------------------------------------------------------|
| `programs/token_factory`     | Ончейн-программа на Anchor: оракул цены + фабрика токенов (см. её [README](programs/token_factory/README.md)) |
| `backend`                    | REST-бэкенд на axum (`/health`, `/oracle`) + опциональный price feed (см. [README](backend/README.md)) |
| `frontend`                   | Preact + TypeScript + Vite, `bun`, подключение кошелька Phantom (см. [README](frontend/README.md)) |
| `scripts/init_oracle`        | CLI для одноразовой инициализации оракула (`make init`, см. [README](scripts/init_oracle/README.md)) |
| `app`                        | Пустая директория (дефолтный Anchor client scaffold), не используется   |

## Toolchain

- Rust `1.89.0`, пиннится через `rust-toolchain.toml` (включает `rustfmt`, `clippy`).
- `anchor-cli` / `anchor-lang` `1.1.2`.
- `solana-cli` (Agave), локальный `~/.config/solana/id.json` как дефолтный кошелёк.
- Frontend — `bun` (не `npm`).

## Быстрый старт (localnet)

Все команды — через `makefile` в корне.

```bash
make install          # зависимости: cargo workspace + frontend (bun install)

# отдельный терминал, не закрывать:
make validator         # solana-test-validator

make build              # anchor build -> target/deploy/token_factory.so + IDL
make deploy             # деплой программы на localnet
make init                # инициализация оракула, печатает ORACLE_STATE_PUBKEY

# скопировать ORACLE_STATE_PUBKEY в backend/.env (см. backend/.env.example),
# затем в отдельных терминалах:
make backend
make frontend
```

```bash
make test               # LiteSVM-тесты программы (oracle + token factory), без сети
```

## Программа `token_factory`

PDA `OracleState` (`admin`, `price`, `decimals`, `last_updated_slot`) обновляется
только админом, с защитой от устаревания и от резких скачков цены. Фабрика токенов
умеет чеканить SPL-минт бесплатно (только админ, `create_token`) или за комиссию
в USD, конвертируемую в SOL по цене оракула и собираемую в PDA-казну `treasury`
(`create_token_with_fee`, доступна любому кошельку). `withdraw_fees` — вывод
накопленной комиссии админом. Подробности инструкций, PDA и ошибок — в
[`programs/token_factory/README.md`](programs/token_factory/README.md).

## Devnet

- Program ID: `9VptbChe2mPw3fkmcnKxCShNmagw4F8YvNX3BzSrPKQF`
- PDA оракула (`OracleState`): `6HKWookZh7eWrBWo9vvp5RUYCHNTSWkKMkfN8QDFNHLB`
- PDA казны (`treasury`): `H7J6HW2iNAaXDTLu9VrKxend92uFWwSwTX956nyTVj9W`
- Ссылки на успешные транзакции создания токенов:
  1. `5xW8YwHJLJjHi1N8bJyDrLFgsPEtDDV4cpsGN3XhpuaqwEqRs7d5ewUhB4rvYtVpLjbSXmT5PBBMNguTwNpUoNtB`
  2. `zcCXQ2hRWFRqwFDPuNgfEhkjiZxm7LutPmshhaDmpSnizY6fwjL6mjb3PyVHms1vdjsWsRNLxcWCjq45XuveNqb`
  3. `FpnMtzciNfLKkbTbZPUBtSc34G3GNj9ie1b62mTX5gYWMwUGZzaeHuPGEV1q4T1dMRhtt36vC2GC6zFgXbY5W68`
