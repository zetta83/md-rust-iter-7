.PHONY: install validator build deploy init backend frontend test

# Зависимости: cargo-крейты воркспейса (backend, programs/*, scripts/*) + frontend (bun)
install:
	cargo fetch
	cd frontend && bun install

# Локальный валидатор Solana. Отдельный терминал, не закрывать.
validator:
	solana-test-validator

# Сборка ончейн-программы (target/deploy/token_factory.so + IDL). Дефолтный формат
# (SBPFv0) — то, что понимает LiteSVM, на котором работает `make test`. rm перед
# anchor build — та же причина, что и в `deploy` ниже: если исходники с прошлой
# сборки не менялись, cargo build-sbf иногда не перекопирует бинарь в target/deploy/
# при смене архитектуры (например, сразу после `make deploy`, который собирает v3).
build:
	rm -f target/deploy/token_factory.so
	anchor build

# Деплой программы в сеть, указанную в Anchor.toml (по умолчанию localnet).
# Локальный solana-test-validator (Agave 4.x) требует SBPFv3 — SIMD-0500 отключает
# деплой v0/v1/v2 программ и не снимается через `--deactivate-feature`, а обычный
# `make build` собирает под v0 (нужно для LiteSVM/`make test`, см. выше). Поэтому
# здесь пересобираем именно под v3 перед деплоем — напрямую через `cargo build-sbf`
# (не `anchor build -- --arch v3`: тот ненадёжно переключает platform-tools и иногда
# не находит sysroot sbpfv3-solana-solana). Сначала удаляем старый target/deploy/*.so:
# у cargo build-sbf при смене --arch иногда не срабатывает копирование нового бинаря
# в target/deploy/, если исходники с прошлой сборки не менялись (кэш по mtime не видит
# разницы в архитектуре) — без rm можно молча задеплоить старый v0-бинарь. Кипейр
# программы (`token_factory-keypair.json`) не трогаем. После деплоя
# `target/deploy/token_factory.so` остаётся в v3-формате — перед следующим `make test`
# нужно снова прогнать `make build`.
deploy:
	rm -f target/deploy/token_factory.so
	cd programs/token_factory && cargo build-sbf --arch v3
	anchor program deploy

# Инициализация оракула (once после деплоя). Печатает ORACLE_STATE_PUBKEY —
# скопируйте его в backend/.env. RPC_URL/PROGRAM_ID/ADMIN_KEYPAIR_PATH/
# INITIALIZE_PRICE можно переопределить через переменные окружения
# (см. scripts/init_oracle/src/main.rs).
init:
	cargo run -p init_oracle

# Backend (нужен backend/.env, см. backend/.env.example)
backend:
	cd backend && cargo run

# Frontend dev-сервер (нужен frontend/.env, см. frontend/.env.example)
frontend:
	cd frontend && bun run dev

# LiteSVM-тесты программы (oracle + token factory), без реальной сети
test:
	cargo test -p token_factory
