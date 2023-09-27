.PHONY: down up test watch setup-static-toolchain push

VERSION = $(shell cat Cargo.toml | grep '^version = "' | grep -o '".*"' | tr -d \")

down:
	docker-compose -f test-services.yaml down

up: 
	docker-compose -f test-services.yaml up

test:
	mkdir -p target
	sqlx migrate run

ifneq (,${FLOWMIUM_INIT_CONTAINER_IMAGE_FROM_SOURCE})
	cargo build
	docker build . -f test-dockerfile -t flowmium-debug

	docker tag flowmium-debug localhost:5180/flowmium-debug:latest
else
	docker pull shnoo28/flowmium:latest
	docker tag shnoo28/flowmium:latest localhost:5180/flowmium-debug:latest
endif
	docker push localhost:5180/flowmium-debug:latest
	KUBECONFIG=./kubeconfig.yaml cargo test

watch:
	KUBECONFIG=./kubeconfig.yaml watch kubectl get pods

setup-static-toolchain:
	rustup target add aarch64-unknown-linux-musl
	rustup target add x86_64-unknown-linux-musl
	cargo install cargo-zigbuild

push:
	rm -rf ../build

	cargo zigbuild --release --target aarch64-unknown-linux-musl
	cargo zigbuild --release --target x86_64-unknown-linux-musl

	mkdir -p ./build/arm64
	mkdir -p ./build/amd64

	cp ./target/aarch64-unknown-linux-musl/release/flowmium ./build/arm64/
	cp ./target/x86_64-unknown-linux-musl/release/flowmium ./build/amd64/

	docker buildx build --push --platform 'linux/arm64,linux/amd64' . -t shnoo28/flowmium:$(VERSION)
	docker buildx build --push --platform 'linux/arm64,linux/amd64' . -t shnoo28/flowmium:latest
