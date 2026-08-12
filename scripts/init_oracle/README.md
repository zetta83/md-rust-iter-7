# init_oracle

Одноразовый CLI-скрипт для инициализации ценового оракула программы
[`token_factory`](../../programs/token_factory) в сети — отправляет инструкцию
`initialize`, создающую PDA `OracleState`. Нужен, потому что до деплоя оракула
`GET /oracle` в [`backend`](../../backend) отвечает `404`, а `create_token`/
`create_token_with_fee` не могут проверить цену.

Обычно запускается не напрямую, а через `make init` (см. корневой `makefile`)
один раз после `make deploy`.

## Что делает

1. Вычисляет адрес PDA оракула (`seeds = [ORACLE_SEED]`, см.
   `programs/token_factory/src/constants.rs`).
2. Если аккаунт по этому адресу уже существует и принадлежит программе —
   ничего не отправляет (повторный `make init` безопасен), печатает текущее
   состояние (`admin`/`price`/`decimals`) в stderr.
3. Иначе отправляет и подтверждает транзакцию `initialize(initialize_price)`,
   подписанную `ADMIN_KEYPAIR_PATH` — этот кошелёк становится `admin`
   оракула.
4. В обоих случаях печатает в **stdout** ровно одну строку:

   ```
   ORACLE_STATE_PUBKEY=<pda>
   ```

   Остальной вывод (статус, подпись транзакции) идёт в stderr, поэтому
   строку из stdout удобно скопировать (или подставить скриптом) в
   `backend/.env`.

Использует тот же способ построения инструкции (`InstructionData` +
`ToAccountMetas` из сгенерированных Anchor-модулей `token_factory::instruction`
/ `token_factory::accounts`, подключённых через `cpi`-фичу), что и
`backend/src/price_feed.rs` и тесты программы (`tests/test_initialize.rs`).

## Переменные окружения

| Переменная           | Обязательна | По умолчанию                    | Описание                                                        |
|-----------------------|:-----------:|----------------------------------|---------------------------------------------------------------------|
| `RPC_URL`             | нет         | `http://127.0.0.1:8899`          | Solana JSON-RPC endpoint                                            |
| `PROGRAM_ID`          | нет         | `token_factory::id()` (id из `declare_id!`) | Program id `token_factory` в целевой сети            |
| `ADMIN_KEYPAIR_PATH`  | нет         | `~/.config/solana/id.json`       | Keypair, который станет `admin` оракула и платит за создание PDA    |
| `INITIALIZE_PRICE`    | нет         | `1_000_000`                      | Начальная цена (масштабирована на `EXPECTED_DECIMALS` = 6, т.е. `1_000_000` = $1.00) |

## Запуск

```bash
# через makefile (из корня репозитория)
make init

# напрямую, с переопределением переменных
RPC_URL=https://api.devnet.solana.com \
ADMIN_KEYPAIR_PATH=~/.config/solana/devnet-admin.json \
INITIALIZE_PRICE=1000000 \
cargo run -p init_oracle
```

Требует уже задеплоенную программу `token_factory` по адресу `PROGRAM_ID` в
сети `RPC_URL` (`make build && make deploy`), и что у `ADMIN_KEYPAIR_PATH`
достаточно SOL для оплаты аренды PDA `OracleState` и комиссии транзакции.
