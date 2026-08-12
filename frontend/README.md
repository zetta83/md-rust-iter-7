# frontend

Минимальный Preact + TypeScript + Vite фронтенд для Mini-Launchpad. Подключает
кошелёк Phantom и проверяет всю цепочку **front → backend → `token_factory`**:
читает состояние оракула через backend (`GET /oracle`) и отправляет ончейн-
транзакцию `create_token_with_fee` напрямую в сеть через `@coral-xyz/anchor`.

## Структура

```
src/
  main.tsx        точка входа, монтирует <App /> в #app
  app.tsx         вся UI-логика: подключение Phantom, обе секции страницы
  app.css         стили страницы
  index.css       глобальные стили
  lib/
    constants.ts    RPC_URL / BACKEND_URL (из VITE_*-переменных) / EXPECTED_DECIMALS
    backend.ts      typed-фетч к backend: fetchHealth(), fetchOracle()
    instructions.ts сборка инструкции createTokenWithFee через Anchor IDL
                     (target/idl/token_factory.json, target/types/token_factory.ts —
                     генерируются `anchor build`, в репозитории не хранятся)
```

## Переменные окружения

Смотри `.env.example`. `.env` в корне `frontend/` подхватывается Vite
автоматически.

| Переменная         | По умолчанию              | Описание                                             |
|----------------------|------------------------------|-----------------------------------------------------------|
| `VITE_RPC_URL`       | `http://127.0.0.1:8899`      | Solana RPC, используется для отправки транзакций напрямую (не через backend) |
| `VITE_BACKEND_URL`   | `http://127.0.0.1:8080`      | Базовый URL backend (`/health`, `/oracle`)                |

`VITE_RPC_URL` также определяет, на какой Explorer-кластер ссылаться после
успешного создания токена (`devnet` / `mainnet` определяются по подстроке в
URL, иначе — `custom` с этим RPC).

## Запуск

```bash
cd frontend
bun install
cp .env.example .env   # при необходимости поправить VITE_RPC_URL/VITE_BACKEND_URL
bun run dev             # dev-сервер Vite
bun run build            # tsc -b && vite build -> dist/
bun run preview          # локальный просмотр собранного dist/
```

Перед первым запуском нужно один раз собрать программу (`anchor build` /
`make build` из корня репозитория) — `lib/instructions.ts` импортирует
сгенерированные `target/idl/token_factory.json` и
`target/types/token_factory.ts` напрямую из воркспейса.

Для работы страницы целиком также нужны поднятый `backend` (см.
[`../backend/README.md`](../backend/README.md)) и установленное расширение
[Phantom](https://phantom.app/) в браузере, переключённое на ту же сеть, что
и `VITE_RPC_URL` (для localnet — Custom RPC `http://127.0.0.1:8899`).

## Страница

Одна страница, header с кнопкой подключения Phantom (`useConnect` /
`useAccounts` / `useSolana` из `@phantom/react-sdk`) и две секции:

**Oracle** — читает `GET /health` и `GET /oracle` у backend, показывает цену
(отмасштабированную по `decimals`), `admin`, `last_updated_slot` и индикатор
`is_stale`; кнопка «Refresh» перезапрашивает оба эндпоинта. Не зависит от
подключённого кошелька.

**Create token** — форма (initial supply, комиссия в USD; `decimals`
фиксирован в `EXPECTED_DECIMALS`, доступна только при подключённом кошельке)
собирает инструкцию `create_token_with_fee` (`lib/instructions.ts`) и
отправляет её через `solana.signAndSendTransaction` из Phantom. После успеха
показывает адрес созданного mint и ссылку на транзакцию в Solana Explorer.
