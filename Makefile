.PHONY: help build build-cli build-ui build-ui-rust build-ui-frontend \
       dev dev-cli dev-ui clean clean-cli clean-ui \
       install run-cli run-ui check release release-fast link-cli

# Default target
help:
	@echo "macrdp Makefile"
	@echo ""
	@echo "  开发:"
	@echo "    make install          安装前端依赖"
	@echo "    make build-cli        编译 CLI"
	@echo "    make dev-ui           启动 UI 开发模式 (Tauri)"
	@echo "    make dev-front        仅启动前端 dev server"
	@echo "    make run-cli          运行 CLI (端口 3389)"
	@echo "    make run-cli-ipc      运行 CLI + IPC socket"
	@echo "    make check            检查所有编译"
	@echo ""
	@echo "  发布:"
	@echo "    make release          完整发布构建 (CLI+UI, 优化, dmg)"
	@echo "    make release-fast     快速发布构建 (无优化, dmg)"
	@echo ""
	@echo "  清理:"
	@echo "    make clean            清理所有构建产物"
	@echo "    make clean-cli        仅清理 CLI"
	@echo "    make clean-ui         仅清理 UI"

# === CLI ===

link-cli:
	@ln -sf ../../target/debug/macrdp-server macrdp-ui/src-tauri/macrdp-server
	@mkdir -p macrdp-ui/src-tauri/target/debug
	@ln -sf ../../../../target/debug/macrdp-server macrdp-ui/src-tauri/target/debug/macrdp-server

build-cli: link-cli
	cargo build -p macrdp-server

run-cli:
	cargo run -p macrdp-server

run-cli-ipc:
	cargo run -p macrdp-server -- --ipc-socket /tmp/macrdp.sock

clean-cli:
	cargo clean

# === UI ===

install:
	cd macrdp-ui && npm install

build-ui-rust:
	cargo build --manifest-path macrdp-ui/src-tauri/Cargo.toml

build-ui-front:
	cd macrdp-ui && npm run build

dev-ui: build-cli
	cd macrdp-ui && npm run tauri dev

dev-front:
	cd macrdp-ui && npm run dev

clean-ui:
	rm -rf macrdp-ui/dist macrdp-ui/src-tauri/target

# === 发布 ===

# 完整发布：全优化编译 + 打包 dmg
release:
	@echo "=== 完整发布构建 (CLI release + UI release) ==="
	MACRDP_CLI_PROFILE=release cd macrdp-ui && npm run tauri build
	@echo ""
	@echo "=== 构建完成 ==="
	@ls -lh macrdp-ui/src-tauri/target/release/bundle/dmg/*.dmg
	@ls -lh macrdp-ui/src-tauri/target/release/bundle/macos/*.app

# 快速发布：关闭优化，加快编译，仍打包 dmg
release-fast:
	@echo "=== 快速发布构建 (CLI debug + UI 无优化) ==="
	MACRDP_CLI_PROFILE=debug cd macrdp-ui && npm run tauri build
	@echo ""
	@echo "=== 构建完成 ==="
	@ls -lh macrdp-ui/src-tauri/target/release/bundle/dmg/*.dmg
	@ls -lh macrdp-ui/src-tauri/target/release/bundle/macos/*.app

# === 全局 ===

build: build-cli build-ui-front build-ui-rust

clean: clean-cli clean-ui

check:
	@echo "检查 CLI..."
	@cargo build -p macrdp-server 2>&1 | tail -1
	@echo "检查 UI Rust..."
	@cargo build --manifest-path macrdp-ui/src-tauri/Cargo.toml 2>&1 | tail -1
	@echo "检查 UI 前端..."
	@cd macrdp-ui && npm run build 2>&1 | tail -1
	@echo "全部通过."
