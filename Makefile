BINARY_NAME ?= yx

.PHONY: build
build:
	go mod tidy
	CGO_ENABLED=0 go build -ldflags '-extldflags "-static" -s -w' -o $(BINARY_NAME) .
