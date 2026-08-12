# token_factory

Ончейн-программа на Anchor для учебного проекта Mini-Launchpad. Она объединяет
простой ценовой оракул (цену обновляет админ, есть проверка на устаревание) и
фабрику SPL-токенов, которая чеканит новые токены бесплатно либо за комиссию в
USD (конвертируется в SOL по цене оракула и собирается в казначейство-PDA).

- Program id: `9VptbChe2mPw3fkmcnKxCShNmagw4F8YvNX3BzSrPKQF`
- Имя крейта / программы: `token_factory`

## Структура

```
src/
  lib.rs                        точка входа программы, объявляет 7 инструкций ниже
  state.rs                      аккаунт OracleState
  constants.rs                  сиды PDA и константы программы (#[constant], попадают в IDL)
  error.rs                      перечисление ErrorCode
  instructions.rs               реэкспорт подмодулей instructions/
  instructions/
    initialize.rs                initialize
    update_price.rs              update_price, get_price
    set_admin.rs                 set_admin
    create_token.rs              create_token, create_token_with_fee
    withdraw_fees.rs             withdraw_fees
tests/                          интеграционные тесты на LiteSVM (живой валидатор не нужен)
```

## Состояние (state)

`OracleState` (PDA, сиды `[ORACLE_SEED]` = `["oracle"]`):

| поле                | тип     | описание                                    |
|---------------------|---------|-----------------------------------------------|
| `admin`             | Pubkey  | адрес, которому разрешено обновлять цену/админа |
| `price`             | u64     | цена 1 SOL в USD, масштабированная под `decimals` |
| `decimals`          | u8      | должно равняться `EXPECTED_DECIMALS` (6)      |
| `last_updated_slot` | u64     | слот последнего вызова `update_price`         |
| `bump`              | u8      | bump PDA                                      |

Прочие PDA:
- Mint authority — сиды `[TOKEN_SEED, mint]` = `["mint_authority", mint]`.
- Treasury (казначейство) — сиды `[TREASURY_SEED]` = `["mint_treasury"]`, обычный
  `SystemAccount`, куда собираются SOL-комиссии из `create_token_with_fee`.

## Инструкции

- **`initialize(initialize_price: u64)`** — создаёт PDA `OracleState`, назначает
  `admin` вызывающего, `decimals` = `EXPECTED_DECIMALS`, `price` = `initialize_price`
  (должна быть `> 0`).

- **`update_price(new_price: u64)`** — только для админа (`has_one = admin`).
  Отклоняет `0`, а после того как цена уже была установлена, ограничивает
  обновления диапазоном ±20% от текущей цены (`ErrorCode::PriceOutOfRange`).
  Обновляет `last_updated_slot` и эмитит событие `PriceUpdated`.

- **`get_price()`** — только чтение, проверка на устаревание: падает с
  `ErrorCode::StaleOracle`, если `текущий_слот - last_updated_slot > MAX_STALENESS_SLOTS`
  (100).

- **`set_admin(new_admin: Pubkey)`** — только для админа, передаёт права админа
  оракула.

- **`create_token(decimals: u8, initial_supply: u64)`** — только для админа
  (у оракула `has_one = admin`). Создаёт новый SPL `Mint` (authority — PDA
  `mint_authority`) и ATA админа, затем чеканит на него
  `initial_supply * 10^decimals` токенов. `decimals` должно равняться
  `EXPECTED_DECIMALS`. Комиссия не взимается.

- **`create_token_with_fee(decimals: u8, initial_supply: u64, fee_usd: u64)`** —
  доступна любому подписанту (`payer`). Проверяет оракул (свежесть, корректные
  decimals, цена > 0), конвертирует `fee_usd` в лампорты по текущей цене
  оракула, переводит соответствующее количество лампортов от `payer` в
  казначейство-PDA, затем чеканит запрошенный supply на ATA плательщика.
  Эмитит событие `TokenCreated`.

- **`withdraw_fees(amount: u64)`** — только для админа. Переводит `amount`
  лампортов из казначейства-PDA обратно `admin`, подписывая транзакцию сидами
  PDA казначейства. Падает, если `amount` равен `0` или превышает баланс
  казначейства.

## Ошибки (`ErrorCode`)

`InvalidPrice`, `StaleOracle`, `PriceOutOfRange`, `BadTokenDecimals`, `BadTokenFeeUsd`,
`BadOracleDecimals`, `MathOverflow`, `InvalidWithdrawAmount`, `InsufficientTreasuryBalance`.

## Константы

| имя                   | значение    |
|------------------------|-------------|
| `ORACLE_SEED`          | `"oracle"`  |
| `TOKEN_SEED`           | `"mint_authority"` |
| `TREASURY_SEED`        | `"mint_treasury"`  |
| `EXPECTED_DECIMALS`    | `6`         |
| `MAX_STALENESS_SLOTS`  | `100`       |
| `LAMPORTS_PER_SOL_U64` | `1_000_000_000` |

## Сборка и тесты

Из корня workspace:

```bash
# собрать программу (получится target/deploy/token_factory.so + IDL)
anchor build

# запустить набор тестов на LiteSVM для этого крейта (живой валидатор не нужен)
anchor test
```
