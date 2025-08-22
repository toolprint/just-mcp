# Real-World Justfile Examples

## Web Development Project

```just
# Node.js project justfile
[doc("Install dependencies and setup development environment")]
setup:
  npm install
  cp .env.example .env
  npm run db:migrate

[doc("Start development server with hot reload")]
dev port="3000":
  npm run dev -- --port {{port}}

[doc("Run linting and formatting")]
lint fix="false":
  npm run lint {{if fix == "true" { "--fix" } else { "" }}}
  npm run prettier {{if fix == "true" { "--write" } else { "--check" }}} .

[doc("Run tests with optional watch mode")]
test watch="false" coverage="false":
  npm test {{ if watch == "true" { "--watch" } else { "" } }} \
           {{ if coverage == "true" { "--coverage" } else { "" } }}

[doc("Build for production with optional analysis")]
build analyze="false":
  npm run build
  {{ if analyze == "true" { "npm run build:analyze" } else { "" } }}

[doc("Deploy to environment")]
deploy env target="":
  #!/usr/bin/env bash
  set -euo pipefail
  echo "Deploying to {{env}}..."
  if [[ "{{target}}" != "" ]]; then
    npm run deploy:{{env}} -- --target {{target}}
  else
    npm run deploy:{{env}}
  fi
```

## Rust Development Project

```just
# Rust project justfile
[doc("Run all checks before committing")]
pre-commit:
  cargo fmt --all -- --check
  cargo clippy -- -D warnings
  cargo test
  cargo doc --no-deps

[doc("Run benchmarks with optional baseline")]
bench baseline="":
  {{ if baseline != "" { "cargo bench -- --baseline " + baseline } else { "cargo bench" } }}

[doc("Generate and open documentation")]
docs open="true":
  cargo doc --no-deps --all-features
  {{ if open == "true" { "open target/doc/$(cargo pkgid | cut -d# -f1 | rev | cut -d/ -f1 | rev)/index.html" } else { "" } }}

[doc("Create a new release")]
release version:
  # Ensure working directory is clean
  git diff-index --quiet HEAD --
  # Update version
  cargo set-version {{version}}
  # Run tests
  cargo test --all-features
  # Commit and tag
  git add Cargo.toml Cargo.lock
  git commit -m "Release v{{version}}"
  git tag -a v{{version}} -m "Release v{{version}}"
  echo "Ready to push: git push && git push --tags"
```

## DevOps/Infrastructure Project

```just
# Infrastructure justfile
[doc("Initialize Terraform workspace")]
tf-init env:
  cd terraform/{{env}} && terraform init -upgrade

[doc("Plan infrastructure changes")]
tf-plan env:
  cd terraform/{{env}} && terraform plan -out=tfplan

[doc("Apply infrastructure changes")]
tf-apply env:
  cd terraform/{{env}} && terraform apply tfplan

[doc("Check Kubernetes cluster health")]
k8s-health context="":
  #!/usr/bin/env bash
  {{ if context != "" { "kubectl config use-context " + context } else { "" } }}
  kubectl cluster-info
  kubectl get nodes
  kubectl get pods --all-namespaces | grep -v Running | grep -v Completed

[doc("Deploy application to Kubernetes")]
k8s-deploy app namespace="default" image_tag="latest":
  kubectl apply -f k8s/{{app}}/namespace.yaml
  kubectl apply -f k8s/{{app}}/config.yaml -n {{namespace}}
  kubectl set image deployment/{{app}} {{app}}={{app}}:{{image_tag}} -n {{namespace}}
  kubectl rollout status deployment/{{app}} -n {{namespace}}

[doc("Stream logs from application")]
logs app namespace="default" follow="true":
  kubectl logs -l app={{app}} -n {{namespace}} {{ if follow == "true" { "-f" } else { "" } }}
```

## Data Science Project

```just
# Data science project justfile
[doc("Setup Python virtual environment")]
venv:
  python -m venv .venv
  .venv/bin/pip install -r requirements.txt
  .venv/bin/pip install -r requirements-dev.txt

[doc("Run Jupyter lab with specific port")]
jupyter port="8888":
  .venv/bin/jupyter lab --port={{port}} --no-browser

[doc("Train model with hyperparameters")]
train model="baseline" epochs="100" batch_size="32":
  .venv/bin/python src/train.py \
    --model {{model}} \
    --epochs {{epochs}} \
    --batch-size {{batch_size}} \
    --output models/{{model}}_{{datetime()}}.pkl

[doc("Evaluate model on test set")]
evaluate model_path dataset="test":
  .venv/bin/python src/evaluate.py \
    --model {{model_path}} \
    --dataset data/{{dataset}}.csv \
    --output reports/evaluation_{{datetime()}}.json

[doc("Generate data quality report")]
data-report input="data/raw" output="reports/data_quality.html":
  .venv/bin/python -m pandas_profiling {{input}} {{output}}
```

## Go Project

```just
# Go project justfile
[doc("Install dependencies and tools")]
setup:
  go mod download
  go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest
  go install golang.org/x/tools/cmd/goimports@latest

[doc("Run tests with coverage")]
test coverage="false":
  go test ./... {{ if coverage == "true" { "-coverprofile=coverage.out" } else { "" } }}
  {{ if coverage == "true" { "go tool cover -html=coverage.out -o coverage.html" } else { "" } }}

[doc("Build for all platforms")]
build-all version="dev":
  #!/usr/bin/env bash
  set -euo pipefail
  mkdir -p dist
  for os in linux darwin windows; do
    for arch in amd64 arm64; do
      if [[ "$os" == "windows" ]]; then
        ext=".exe"
      else
        ext=""
      fi
      echo "Building $os/$arch..."
      GOOS=$os GOARCH=$arch go build -o "dist/myapp-$os-$arch$ext" \
        -ldflags "-X main.Version={{version}}" .
    done
  done

[doc("Run linting")]
lint fix="false":
  goimports {{ if fix == "true" { "-w" } else { "-d" } }} .
  golangci-lint run {{ if fix == "true" { "--fix" } else { "" } }}

[doc("Run database migrations")]
migrate direction="up" steps="":
  #!/usr/bin/env bash
  if [[ "{{steps}}" != "" ]]; then
    go run cmd/migrate/main.go {{direction}} {{steps}}
  else
    go run cmd/migrate/main.go {{direction}}
  fi
```

## Docker Multi-Service Project

```just
# Docker compose project
[doc("Start all services")]
up service="":
  {{ if service != "" { "docker compose up -d " + service } else { "docker compose up -d" } }}

[doc("View logs for services")]
logs service="" follow="true":
  docker compose logs {{ if follow == "true" { "-f" } else { "" } }} {{ service }}

[doc("Run database migrations")]
migrate:
  docker compose exec api npm run migrate

[doc("Run tests in containers")]
test service="":
  {{ if service != "" { "docker compose exec " + service + " npm test" } else { "docker compose exec api npm test && docker compose exec worker npm test" } }}

[doc("Build and push images")]
push tag="latest":
  docker compose build
  docker compose push

[doc("Scale services")]
scale service replicas="3":
  docker compose up -d --scale {{service}}={{replicas}}

[doc("Health check all services")]
health:
  #!/usr/bin/env bash
  services=$(docker compose ps --services)
  for service in $services; do
    status=$(docker compose ps $service --format "table {{.Service}}\t{{.State}}" | tail -n +2)
    echo "$status"
  done
```