.PHONY: build test deploy-btc deploy-ada monitor clean

build:
	cargo build --release
	cd cardano && cabal build PQCValidator.hs

test:
	cargo test --lib
	anchor test --skip-local-validator --bpf-program pqc_bridge anchor/programs/pqc_bridge/src/lib.rs

deploy-btc:
	bitcoin-cli signrawtransactionwithwallet "$(PSBT_HEX)"
	bitcoin-cli sendrawtransaction "$(RAW_TX_HEX)"

deploy-ada:
	cardano-cli transaction build \
		--script-file cardano/PQCValidator.plutus \
		--testnet-magic 0 \
		--out-file pqc_claim.tx
	cardano-cli transaction submit --testnet-magic 0 pqc_claim.tx

monitor:
	docker-compose up -d prometheus grafana

clean:
	cargo clean
	rm -rf target/ cardano/dist/