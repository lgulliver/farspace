.PHONY: run test lint

run:
	go run ./cmd/farspace

test:
	go test ./...

lint:
	@echo "lint not implemented"
