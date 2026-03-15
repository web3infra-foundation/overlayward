# Overlayward

AI Agent 全动作安全沙箱 | AI Agent Full-Action Security Sandbox

[中文](#中文) | [English](#english)

---

<a id="中文"></a>

## 中文

### 简介

Overlayward 是一个让 AI 编程 Agent 的所有操作都在完全隔离环境中运行的安全沙箱系统。

**当前功能：**

- 5 服务微服务架构（ow-gateway / ow-policy / ow-sandbox / ow-audit / ow-data）
- 4 种接入协议：REST API / HTTP/3 (QUIC) / MCP (stdio + Streamable HTTP) / CLI
- 完整的认证和权限系统（Agent / User / Admin / Human 四级）
- 沙箱生命周期管理（创建 / 启动 / 暂停 / 恢复 / 停止 / 销毁）
- 快照系统（保存 / 恢复 / 列表 / 差异比较）
- 命令执行、文件读写、目录列表
- 网络策略引擎（白名单放行 / 内网拒绝 / 未知域名触发人类审批）
- 资源监控（CPU / 内存 / 磁盘 / GPU）
- 共享卷管理、沙箱间通信
- 审计日志查询与操作回放
- 审批流程（人类审批门）
- 服务发现 + 心跳检测
- 两种部署模式：完整部署（serve-all）和最小部署（仅 ow-sandbox）
- ow-sandbox 独立二合一工具（服务器 + CLI 客户端，一个二进制搞定最小部署）
- 19 个 MCP Tool 完整实现

当前为 Mock 阶段，所有 API 返回模拟数据。项目按生产架构设计，后续逐步替换为真实后端。

### 编译

#### 前置条件

- Rust 1.75+（推荐 1.94+）

#### Windows

```powershell
# 安装 Rust（如果未安装）
winget install Rustlang.Rust.MSVC
# 或从 https://rustup.rs 下载安装

# 编译
cargo build

# Release 编译（LTO 优化，二进制更小更快）
cargo build --release

# 产物位置
.\target\debug\overlayward.exe      # Debug
.\target\release\overlayward.exe    # Release
```

#### macOS

```bash
# 安装 Rust（如果未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 编译
cargo build

# Release 编译
cargo build --release

# 产物位置
./target/debug/overlayward
./target/release/overlayward
```

#### Linux

```bash
# 安装 Rust（如果未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装编译依赖（Debian/Ubuntu）
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev

# 编译
cargo build

# Release 编译
cargo build --release

# 产物位置
./target/debug/overlayward
./target/release/overlayward
```

#### 编译产物

编译生成 6 个可执行文件：

| 文件 | 用途 |
|------|------|
| `overlayward` | 统一入口：serve-all / mcp-server / CLI 客户端 |
| `ow-gateway` | API 网关（REST :8420 + HTTP/3 :8425 + MCP :8426） |
| `ow-policy` | 策略引擎（:8421） |
| `ow-sandbox` | 沙箱引擎（:8422）— 自包含二合一：服务器 + CLI 客户端 |
| `ow-audit` | 审计系统（:8423） |
| `ow-data` | 数据交换（:8424） |

### 使用方法

#### 完整部署（一键启动全部 5 个服务）

```bash
RUST_LOG=info overlayward serve
```

输出：
```
INFO overlayward: Overlayward serve-all started — 5 services running
INFO ow_sandbox: ow-sandbox listening on 0.0.0.0:8422
INFO ow_audit:   ow-audit listening on 0.0.0.0:8423
INFO ow_data:    ow-data listening on 0.0.0.0:8424
INFO ow_policy:  ow-policy listening on 0.0.0.0:8421
INFO ow_gateway: Overlayward Gateway started — REST :8420 | H3 :8425 | MCP :8426
```

也可独立启动各服务：
```bash
ow-sandbox &     # :8422
ow-audit &       # :8423
ow-data &        # :8424
ow-policy &      # :8421（心跳检测 sandbox/audit/data）
ow-gateway &     # :8420（心跳检测 policy）
```

#### 最小部署（ow-sandbox 独立二合一）

`ow-sandbox` 是自包含的二合一工具，既能当服务器，又能当 CLI 客户端。最小部署只需这一个二进制：

```bash
# 终端 1: 启动沙箱引擎
ow-sandbox serve --port 8422

# 终端 2: 直接操作（连 localhost:8422，无需 Token，无需 overlayward）
ow-sandbox create --name test --cpu 2 --memory 4GB
ow-sandbox list
ow-sandbox start sb-xxx
ow-sandbox exec sb-xxx -- npm install express
ow-sandbox snapshot save sb-xxx --name checkpoint
ow-sandbox snapshot list sb-xxx
ow-sandbox resource usage sb-xxx
ow-sandbox info sb-xxx
ow-sandbox stop sb-xxx
ow-sandbox destroy sb-xxx

# JSON 输出
ow-sandbox --output json list

# 自定义连接地址
ow-sandbox --endpoint http://192.168.1.10:8422 list
```

不需要 `overlayward --direct`，`ow-sandbox` 本身就是完整的最小部署工具。不暴露审计、审批、网络策略、共享卷等命令（这些属于完整部署）。

`overlayward --direct` 仍然可用，效果相同。

#### 健康检查

每个服务暴露 `GET /healthz`：
```bash
curl http://localhost:8422/healthz
# {"service":"ow-sandbox","status":"ok","port":8422}
```

#### 认证

通过 Gateway（:8420）访问需要 Bearer Token：

| Token | 角色 | 可用操作 |
|-------|------|---------|
| `ow-agent-token` | Agent | 沙箱操作、文件、执行、快照、网络查看 |
| `ow-user-token` | User | 以上 + 审计查询、文件上传下载、资源调整 |
| `ow-admin-token` | Admin | 以上 + 网络默认策略修改 |
| `ow-human-token` | Human | 全部（含审批决策） |

直连 ow-sandbox（最小部署）不需要 Token。

#### REST API

```bash
# 创建沙箱
curl -X POST http://localhost:8420/api/v1/sandboxes \
  -H "Authorization: Bearer ow-agent-token" \
  -H "Content-Type: application/json" \
  -d '{"name":"dev","cpu":4,"memory":"8GB"}'

# 列出 / 详情
curl -H "Authorization: Bearer ow-agent-token" http://localhost:8420/api/v1/sandboxes
curl -H "Authorization: Bearer ow-agent-token" http://localhost:8420/api/v1/sandboxes/sb-xxx

# 生命周期
curl -X POST http://localhost:8420/api/v1/sandboxes/sb-xxx/start -H "Authorization: Bearer ow-agent-token"
curl -X POST http://localhost:8420/api/v1/sandboxes/sb-xxx/pause -H "Authorization: Bearer ow-agent-token"
curl -X POST http://localhost:8420/api/v1/sandboxes/sb-xxx/resume -H "Authorization: Bearer ow-agent-token"
curl -X POST http://localhost:8420/api/v1/sandboxes/sb-xxx/stop -H "Authorization: Bearer ow-agent-token"
curl -X DELETE http://localhost:8420/api/v1/sandboxes/sb-xxx -H "Authorization: Bearer ow-agent-token"

# 执行命令
curl -X POST http://localhost:8420/api/v1/sandboxes/sb-xxx/exec \
  -H "Authorization: Bearer ow-agent-token" \
  -H "Content-Type: application/json" \
  -d '{"sandbox_id":"sb-xxx","command":"npm install express"}'

# 快照
curl -X POST http://localhost:8420/api/v1/sandboxes/sb-xxx/snapshots \
  -H "Authorization: Bearer ow-agent-token" \
  -H "Content-Type: application/json" \
  -d '{"name":"checkpoint"}'

# 网络策略
curl -H "Authorization: Bearer ow-agent-token" http://localhost:8420/api/v1/sandboxes/sb-xxx/network

# 资源使用
curl -H "Authorization: Bearer ow-agent-token" http://localhost:8420/api/v1/sandboxes/sb-xxx/resources
```

完整路由表（38 条）见 `src/ow-gateway/src/routes/`。

#### CLI

```bash
export OVERLAYWARD_TOKEN=ow-agent-token

# 沙箱管理
overlayward create --name dev --cpu 4 --memory 8GB
overlayward list
overlayward start sb-xxx
overlayward info sb-xxx
overlayward stop sb-xxx
overlayward destroy sb-xxx

# 执行命令
overlayward exec sb-xxx -- npm install express

# 快照
overlayward snapshot save sb-xxx --name checkpoint
overlayward snapshot list sb-xxx

# 网络 / 资源 / 审计
overlayward network get sb-xxx
overlayward resource usage sb-xxx
overlayward --token ow-user-token audit query sb-xxx --level command
overlayward --token ow-human-token approval list

# JSON 输出
overlayward --output json list

# 直连模式（最小部署，连 ow-sandbox :8422，不需要 Token）
overlayward --direct create --name test
overlayward --direct exec sb-xxx -- cargo build
```

#### MCP

**Streamable HTTP（:8426）：**
```json
{ "mcpServers": { "overlayward": { "url": "http://localhost:8426/mcp" } } }
```

**stdio：**
```json
{ "mcpServers": { "overlayward": { "command": "overlayward", "args": ["mcp-server"] } } }
```

19 个 Tool：overlayward_create / start / stop / destroy / list / info / exec / file_read / file_write / file_list / snapshot_save / snapshot_restore / snapshot_list / snapshot_diff / network_get / network_allow / resource_usage / volume_list / inter_send

#### HTTP/3 (QUIC)

Gateway 同时在 UDP 端口 8425 提供 HTTP/3 over QUIC，共享与 REST 相同的路由和认证。开发环境自动生成自签名证书。

```bash
# 需要支持 HTTP/3 的 curl（7.88+，编译时带 --with-ngtcp2 或 --with-quiche）
curl --http3-only -k https://localhost:8425/healthz
curl --http3-only -k -H "Authorization: Bearer ow-agent-token" https://localhost:8425/api/v1/sandboxes

# 自定义端口
overlayward serve --h3-port 9425
```

> 注意：`-k` 跳过自签名证书验证。生产环境应使用正式 TLS 证书。

### 项目结构

```
overlayward/
├── src/
│   ├── main.rs                 # 统一入口
│   ├── bin/                    # 5 个独立服务 binary
│   ├── ow-service-common/      # 服务公共（健康检查 / 心跳 / 发现）
│   ├── ow-gateway/             # API 网关（REST + HTTP/3 + MCP + Mock）
│   ├── ow-policy/              # 策略引擎（Guardian + 审批）
│   ├── ow-sandbox/             # 沙箱引擎（VM + 执行 + 文件 + 快照）
│   ├── ow-audit/               # 审计（日志 + 回放 + 事件）
│   ├── ow-data/                # 数据交换（卷 + 网络 + 沙箱间通信）
│   ├── ow-types/               # 领域模型 + 错误码
│   ├── ow-cli/                 # CLI 客户端
│   └── ow-macros/              # 过程宏
├── overlayward.yaml            # 服务发现配置
```

### 服务端口

| 服务 | 端口 | 说明 |
|------|------|------|
| ow-gateway REST | 8420 | REST API + 健康检查 |
| ow-policy | 8421 | 策略引擎 + 健康检查 |
| ow-sandbox | 8422 | 沙箱引擎 + 简化 REST + 健康检查 |
| ow-audit | 8423 | 审计系统 + 健康检查 |
| ow-data | 8424 | 数据交换 + 健康检查 |
| ow-gateway HTTP/3 | 8425 | HTTP/3 over QUIC (UDP) |
| ow-gateway MCP | 8426 | MCP Streamable HTTP |

---

<a id="english"></a>

## English

### Introduction

Overlayward is a security sandbox system that runs all AI programming Agent operations in fully isolated environments.

**Current Features:**

- 5-service microservice architecture (ow-gateway / ow-policy / ow-sandbox / ow-audit / ow-data)
- 4 access protocols: REST API / HTTP/3 (QUIC) / MCP (stdio + Streamable HTTP) / CLI
- Full authentication and permission system (Agent / User / Admin / Human, 4 levels)
- Sandbox lifecycle management (create / start / pause / resume / stop / destroy)
- Snapshot system (save / restore / list / diff)
- Command execution, file read/write, directory listing
- Network policy engine (whitelist allow / internal IP deny / unknown domain triggers human approval)
- Resource monitoring (CPU / memory / disk / GPU)
- Shared volume management, inter-sandbox communication
- Audit log query and operation replay
- Approval workflow (human approval gate)
- Service discovery + heartbeat detection
- Two deployment modes: full (serve-all) and minimal (ow-sandbox only)
- ow-sandbox standalone 2-in-1 tool (server + CLI client, single binary for minimal deployment)
- 19 MCP Tools fully implemented

Currently in Mock stage — all APIs return simulated data. Production architecture in place for gradual backend replacement.

### Building

#### Prerequisites

- Rust 1.75+ (1.94+ recommended)

#### Windows

```powershell
# Install Rust (if not installed)
winget install Rustlang.Rust.MSVC
# Or download from https://rustup.rs

# Build
cargo build

# Release build (LTO optimized)
cargo build --release

# Output
.\target\debug\overlayward.exe      # Debug
.\target\release\overlayward.exe    # Release
```

#### macOS

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cargo build

# Release build
cargo build --release
```

#### Linux

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies (Debian/Ubuntu)
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev

# Build
cargo build

# Release build
cargo build --release
```

#### Build Artifacts

6 executables are produced:

| File | Purpose |
|------|---------|
| `overlayward` | Unified entry: serve-all / mcp-server / CLI client |
| `ow-gateway` | API Gateway (REST :8420 + HTTP/3 :8425 + MCP :8426) |
| `ow-sandbox` | Sandbox Engine (:8422) -- self-contained 2-in-1: server + CLI client |
| `ow-policy` | Policy Engine (:8421) |
| `ow-audit` | Audit System (:8423) |
| `ow-data` | Data Exchange (:8424) |

### Usage

#### Full Deployment (start all 5 services)

```bash
RUST_LOG=info overlayward serve
```

Or start each service independently:
```bash
ow-sandbox &     # :8422
ow-audit &       # :8423
ow-data &        # :8424
ow-policy &      # :8421 (heartbeat checks sandbox/audit/data)
ow-gateway &     # :8420 (heartbeat checks policy)
```

#### Minimal Deployment (ow-sandbox standalone 2-in-1)

`ow-sandbox` is a self-contained 2-in-1 tool -- both server and CLI client. Only this single binary is needed for minimal deployment:

```bash
# Terminal 1: Start sandbox engine
ow-sandbox serve --port 8422

# Terminal 2: Operate directly (connects localhost:8422, no token, no overlayward needed)
ow-sandbox create --name test --cpu 2 --memory 4GB
ow-sandbox list
ow-sandbox start sb-xxx
ow-sandbox exec sb-xxx -- npm install express
ow-sandbox snapshot save sb-xxx --name checkpoint
ow-sandbox snapshot list sb-xxx
ow-sandbox resource usage sb-xxx
ow-sandbox info sb-xxx
ow-sandbox stop sb-xxx
ow-sandbox destroy sb-xxx

# JSON output
ow-sandbox --output json list

# Custom endpoint
ow-sandbox --endpoint http://192.168.1.10:8422 list
```

No need for `overlayward --direct` -- `ow-sandbox` itself is the complete minimal deployment tool. Audit, approval, network policy, and shared volume commands are not exposed (they require full deployment).

`overlayward --direct` still works as an alternative.

#### Health Check

Every service exposes `GET /healthz`:
```bash
curl http://localhost:8422/healthz
# {"service":"ow-sandbox","status":"ok","port":8422}
```

#### Authentication

Access through Gateway (:8420) requires a Bearer Token:

| Token | Role | Available Operations |
|-------|------|---------------------|
| `ow-agent-token` | Agent | Sandbox ops, files, exec, snapshots, network view |
| `ow-user-token` | User | Above + audit query, file upload/download, resize |
| `ow-admin-token` | Admin | Above + network default policy |
| `ow-human-token` | Human | All (including approval decisions) |

Direct connection to ow-sandbox (minimal deployment) requires no token.

#### REST API

```bash
# Create sandbox
curl -X POST http://localhost:8420/api/v1/sandboxes \
  -H "Authorization: Bearer ow-agent-token" \
  -H "Content-Type: application/json" \
  -d '{"name":"dev","cpu":4,"memory":"8GB"}'

# List / Info
curl -H "Authorization: Bearer ow-agent-token" http://localhost:8420/api/v1/sandboxes
curl -H "Authorization: Bearer ow-agent-token" http://localhost:8420/api/v1/sandboxes/sb-xxx

# Lifecycle
curl -X POST .../sandboxes/sb-xxx/start -H "Authorization: Bearer ow-agent-token"
curl -X POST .../sandboxes/sb-xxx/pause -H "Authorization: Bearer ow-agent-token"
curl -X POST .../sandboxes/sb-xxx/resume -H "Authorization: Bearer ow-agent-token"
curl -X POST .../sandboxes/sb-xxx/stop -H "Authorization: Bearer ow-agent-token"
curl -X DELETE .../sandboxes/sb-xxx -H "Authorization: Bearer ow-agent-token"

# Execute command
curl -X POST .../sandboxes/sb-xxx/exec \
  -H "Authorization: Bearer ow-agent-token" \
  -H "Content-Type: application/json" \
  -d '{"sandbox_id":"sb-xxx","command":"npm install"}'

# Snapshot / Network / Resources
curl -X POST .../sandboxes/sb-xxx/snapshots -H "..." -d '{"name":"checkpoint"}'
curl .../sandboxes/sb-xxx/network -H "..."
curl .../sandboxes/sb-xxx/resources -H "..."
```

38 routes total. See `src/ow-gateway/src/routes/` for full list.

#### CLI

```bash
export OVERLAYWARD_TOKEN=ow-agent-token

overlayward create --name dev --cpu 4 --memory 8GB
overlayward list
overlayward start sb-xxx
overlayward exec sb-xxx -- npm install express
overlayward snapshot save sb-xxx --name checkpoint
overlayward network get sb-xxx
overlayward resource usage sb-xxx
overlayward --output json list

# Direct mode (minimal deployment, connects to ow-sandbox :8422, no token)
overlayward --direct create --name test
overlayward --direct exec sb-xxx -- cargo build
```

#### MCP

**Streamable HTTP (:8426):**
```json
{ "mcpServers": { "overlayward": { "url": "http://localhost:8426/mcp" } } }
```

**stdio:**
```json
{ "mcpServers": { "overlayward": { "command": "overlayward", "args": ["mcp-server"] } } }
```

19 Tools: overlayward_create / start / stop / destroy / list / info / exec / file_read / file_write / file_list / snapshot_save / snapshot_restore / snapshot_list / snapshot_diff / network_get / network_allow / resource_usage / volume_list / inter_send

#### HTTP/3 (QUIC)

The Gateway also serves HTTP/3 over QUIC on UDP port 8425, sharing the same routes and auth as REST. A self-signed certificate is auto-generated for development.

```bash
# Requires HTTP/3-capable curl (7.88+, built with --with-ngtcp2 or --with-quiche)
curl --http3-only -k https://localhost:8425/healthz
curl --http3-only -k -H "Authorization: Bearer ow-agent-token" https://localhost:8425/api/v1/sandboxes

# Custom port
overlayward serve --h3-port 9425
```

> Note: `-k` skips self-signed certificate verification. Use proper TLS certificates in production.

### Project Structure

```
overlayward/
├── src/
│   ├── main.rs                 # Unified entry
│   ├── bin/                    # 5 standalone service binaries
│   ├── ow-service-common/      # Service commons (health / heartbeat / discovery)
│   ├── ow-gateway/             # API Gateway (REST + HTTP/3 + MCP + Mock)
│   ├── ow-policy/              # Policy Engine (Guardian + Approval)
│   ├── ow-sandbox/             # Sandbox Engine (VM + Exec + Files + Snapshots)
│   ├── ow-audit/               # Audit (Logs + Replay + Events)
│   ├── ow-data/                # Data Exchange (Volumes + Network + IPC)
│   ├── ow-types/               # Domain models + Error codes
│   ├── ow-cli/                 # CLI client
│   └── ow-macros/              # Proc macros
├── overlayward.yaml            # Service discovery config
```

### Service Ports

| Service | Port | Description |
|---------|------|-------------|
| ow-gateway REST | 8420 | REST API + Health check |
| ow-policy | 8421 | Policy engine + Health check |
| ow-sandbox | 8422 | Sandbox engine + Simplified REST + Health check |
| ow-audit | 8423 | Audit system + Health check |
| ow-data | 8424 | Data exchange + Health check |
| ow-gateway HTTP/3 | 8425 | HTTP/3 over QUIC (UDP) |
| ow-gateway MCP | 8426 | MCP Streamable HTTP |

---

## License

MIT
