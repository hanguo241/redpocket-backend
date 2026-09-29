# syntax=docker/dockerfile:1

# =============================================
# RedPacket 后端镜像
# =============================================
# 构建阶段: rust 官方 Debian 镜像（官方没有 Ubuntu 版 Rust 镜像）
# 运行阶段: Ubuntu
#
# 为什么这样混搭是安全的：二进制链接的是构建机的 glibc 版本，运行机的 glibc
# 只要 >= 它就行。Debian bookworm = glibc 2.36，Ubuntu 24.04 = glibc 2.39 ✓
# ⚠️ 别把运行镜像降到 ubuntu:22.04（glibc 2.35 < 2.36，二进制会跑不起来）
#
# 服务器自己是什么发行版不影响容器：容器自带整套用户态。
# =============================================

# ---------------- 构建 ----------------
# 没钉死具体版本号：首次构建成功后，用 `docker run --rm <image> rustc --version`
# 查到实际版本再钉（例如 rust:1.93-bookworm），保证以后可复现。
FROM rust:bookworm AS builder

WORKDIR /app

# cargo 缓存挂在 registry 和 target 上：
# 改源码重新构建时只重编本 crate，不会把 ethers / axum / sqlx 那一堆依赖重编一遍。
# 注意 cache mount 不进镜像层，所以二进制必须在同一个 RUN 里拷到 /out 去。
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bins && \
    mkdir -p /out && \
    cp target/release/redpacket-backend target/release/redpacket-sync-worker /out/

# ---------------- 运行 ----------------
FROM ubuntu:24.04

# ca-certificates: 访问 HTTPS RPC 必需（没有根证书，链上调用全报证书错误）
# openssl:         reqwest 走 native-tls，二进制动态链接 libssl.so.3
#                  （Cargo.lock 里有 openssl-sys）—— 装 openssl 包，
#                  它会把对应版本的运行库一起带进来，不用纠结 libssl3 / libssl3t64 的包名差异
# curl:            容器 healthcheck 用
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates openssl curl && \
    rm -rf /var/lib/apt/lists/*

# 非 root 运行
RUN useradd --system --uid 10001 --create-home --home-dir /opt/redpacket-backend redpacket

WORKDIR /opt/redpacket-backend
COPY --from=builder --chown=redpacket:redpacket \
    /out/redpacket-backend /out/redpacket-sync-worker ./

USER redpacket
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1

# 默认起 API；sync-worker 由 compose 用 command 覆盖
CMD ["./redpacket-backend"]
