# Ubuntu 26.04 部署约定

本仓库的原生部署采用 `scripts/00-check-env.sh` 至 `scripts/06-configure-nginx.sh` 分步入口，`scripts/deploy.sh` 仅按顺序编排。共享检查放在 `scripts/deploy-common.sh`，不单独执行。每个步骤只负责自身工作，缺依赖时报错，不隐式安装其他步骤的依赖。在服务器 checkout 中执行。脚本和 systemd 模板放在 `scripts/`，不依赖父级多项目仓库。源码和 Rust 工具链属于登录用户；运行程序使用独立的 `redpacket` 系统用户。配置保存在 `/etc/redpacket/backend.env`（root:root，0600），程序保存在 `/opt/redpacket-backend/bin`。不将密码或私钥写进仓库、命令参数或日志。

只支持 Ubuntu 26.04。使用系统 PostgreSQL、Nginx、rustup stable 和 SQLx CLI 0.8.6。数据库迁移由 SQLx 记录版本，不修改历史迁移、不全量重放。已有数据库必须已有 SQLx 迁移记录；旧的手工 SQL 数据库需单独规划接管。

验证要求：Shell 语法检查、可用时 ShellCheck、参数/平台拒绝路径检查；完整安装需在 Ubuntu 26.04 测试机验证，不能在开发者 macOS 上执行安装阶段。部署失败必须非零退出，健康检查通过后才报告成功。


## 首次部署

以 Ubuntu 默认的 `ubuntu` 用户登录（不要用 root clone）。第一步只需 Git；其余依赖由脚本安装：

```bash
sudo apt-get update
sudo apt-get install -y git
git clone https://github.com/hanguo241/redpocket-backend.git
cd redpocket-backend
# 检查已有环境，不安装软件；缺项时退出码为 1
sudo bash scripts/00-check-env.sh

# 只在缺少环境时执行，可跳过
sudo bash scripts/01-install-env.sh

# 每个步骤独立执行
sudo bash scripts/02-init-db.sh
sudo bash scripts/03-migrate-db.sh
sudo bash scripts/04-build.sh
sudo bash scripts/05-configure-services.sh
sudo bash scripts/06-configure-nginx.sh api.example.com
```

把 `api.example.com` 换成 API 域名，也可以先用服务器公网 IP。私有仓库使用你自己的 GitHub SSH/PAT 登录方式，不把 token 写在 clone URL。脚本必须已经提交到你 clone 的分支；本地修改不会自动出现在 GitHub。

| 脚本 | 职责 | 何时可跳过 |
| --- | --- | --- |
| `00-check-env.sh` | 检查软件包、登录用户的 Rust/SQLx、PostgreSQL 连接 | 已确认环境时 |
| `01-install-env.sh` | 安装系统依赖、Rust、SQLx | 环境已准备好时 |
| `02-init-db.sh` | 创建数据库用户/数据库，生成生产配置 | 数据库和生产配置都已准备好时 |
| `03-migrate-db.sh` | SQLx 版本化迁移；首次初始化设置管理员密码 | 当前版本迁移已经完成时 |
| `04-build.sh` | 编译两个二进制，不替换运行程序 | 已有当前版本的服务器构建产物时 |
| `05-configure-services.sh` | 创建运行用户，安装二进制和 systemd 服务，重启并检查 | 不需要更新运行程序时 |
| `06-configure-nginx.sh` | 配置 Nginx 并重新加载，保留已有站点/TLS配置 | 已配置代理时 |

所有脚本支持 `--help`，需要以同一个登录用户执行 `sudo bash ...`。`deploy-common.sh` 是共享函数，不要单独执行。步骤缺依赖时会报错，不会偷偷安装软件。

只安装一类环境：

```bash
sudo bash scripts/01-install-env.sh system  # apt 软件包和 PostgreSQL 启动
sudo bash scripts/01-install-env.sh rust    # 登录用户的 rustup + stable
sudo bash scripts/01-install-env.sh sqlx    # SQLx CLI 0.8.6，需要已有 Rust
```

`system` 会对列出的包执行 apt 安装，可能升级已安装包。若只缺个别系统包，按检查输出自行 `sudo apt-get install 包名` 即可。检查脚本面向 Ubuntu 官方包和 rustup stable；其他方式安装的等价环境可能被报告为缺少，需自行核对。PostgreSQL 需在本机运行并允许 postgres 系统用户通过 socket 访问。

低内存服务器可指定单任务编译：

```bash
sudo bash scripts/04-build.sh 1
```

已有数据库时，不要重复创建或更换密码：在 `/etc/redpacket/backend.env` 准备生产配置，设置 `root:root` 和 `0600` 后，跳过 `02-init-db.sh`。当前脚本约定本机数据库名为 `redpacket`，配置使用该库的连接串，API 地址固定为 `127.0.0.1:8080`。环境变量清单见仓库 `.env.example`，必须填入真实生产值。已有库必须有正确的 SQLx 迁移记录；没有记录的手工初始化库会被拒绝。接管已有配置时不自动重设管理员密码，需保证默认管理员密码已更换。

仍然保留一键入口（全新机器安装）：

```bash
sudo bash scripts/deploy.sh install api.example.com
```

需要交互输入平台签名私钥、独立 relayer 私钥、官网 HTTPS 地址和新的管理员密码。已有业务必须使用原平台签名密钥，relayer 钱包需有 gas。脚本不读取或复制开发机 `.env`。数据库密码及 JWT 保存在服务器 `/etc/redpacket/backend.env`，不会打印。

服务器需要能访问 Ubuntu 软件源、GitHub、rustup、crates.io 及链 RPC。编译资源有限时使用 `sudo bash scripts/04-build.sh 1`。`install` 可重试：已生成的配置不会覆盖；初始化标记允许数据库创建或密码输入中断后继续。已有数据库用户时输入其当前密码并验证连接，不重置密码；已有数据库也可复用，不改变 owner 或已有管理员密码。密码中的特殊字符会做 URL 编码。缺少原密码时停止，不自动修改数据库凭据。

## HTTPS 与网络

安全组放行 80/443 和限定来源的 SSH 22；无需对外开放 5432 或 8080。脚本不改安全组、UFW 或 PostgreSQL 网络设置。若 UFW 已启用，需允许 Nginx Full。保留现有 Nginx 站点，使用指定域名/IP匹配本项目。

首次安装先提供 HTTP。域名 A 记录指向服务器、80/443 可达后运行：

```bash
sudo apt-get install -y certbot python3-certbot-nginx
sudo certbot --nginx -d api.example.com
sudo certbot renew --dry-run
curl --fail https://api.example.com/health
```

官网使用 HTTPS 时，前端配置的 API 地址也需要 HTTPS。后续 `update` 和重复 `install` 都保留已存在的 Nginx 站点文件，不覆盖证书设置。若要更换域名，编辑 `/etc/nginx/sites-available/redpacket-backend` 后执行 `sudo nginx -t && sudo systemctl reload nginx`。

## 更新与备份

在服务器仓库中执行，先备份再迁移：

```bash
sudo install -d -m 700 /var/backups/redpacket
sudo bash -c 'umask 077; runuser -u postgres -- pg_dump -Fc redpacket > /var/backups/redpacket/redpacket-$(date +%Y%m%d-%H%M%S).dump'
git pull --ff-only
sudo bash scripts/deploy.sh update
```

也可以逐步更新，已有环境和 Nginx 不需要重复配置：

```bash
sudo bash scripts/03-migrate-db.sh
sudo bash scripts/04-build.sh
sudo bash scripts/05-configure-services.sh
```

每个步骤独立持有部署锁；不要同时运行两个一键部署流程或在步骤之间并行更新 checkout。

更新按“迁移 → 编译 → 替换二进制 → 重启”执行；构建失败时原服务继续运行，但已完成的迁移不会自动撤销。新迁移必须兼容正在运行的旧版本；不兼容迁移需提前安排停机。二进制更新和两个服务重启不是原子切换，会短暂停机。回滚代码前需核对数据库兼容性，不能把恢复旧二进制当成数据库回滚。

worker 本身也使用 SQLx 自动检查迁移，与 CLI 共用 `_sqlx_migrations`。不要修改已经应用的 SQL 文件；不要运行父仓库旧的“全量 SQL 重放”脚本。

## 运维命令

```bash
sudo systemctl status redpacket-backend redpacket-sync-worker --no-pager
sudo systemctl restart redpacket-backend redpacket-sync-worker
sudo systemctl stop redpacket-backend redpacket-sync-worker
sudo systemctl start redpacket-backend redpacket-sync-worker
sudo journalctl -u redpacket-backend -u redpacket-sync-worker -n 100 --no-pager
sudo journalctl -u redpacket-backend -f
sudo nginx -t
curl --fail http://127.0.0.1:8080/health
sudo -u postgres psql -d redpacket
sudoedit /etc/redpacket/backend.env
```

配置使用简单 `KEY=value` 格式，兼容 Bash 与 systemd，不添加 shell 命令。修改配置后重启两个服务。`/health` 仅验证 HTTP 进程存活，不代表链 RPC、钱包余额或全部业务可用；上线前需验证后台登录、链配置和领取流程。

## 验证范围

本次开发环境为 macOS：可完成脚本语法、控制流和模板检查，不能证明 Ubuntu 26.04 软件源安装、Rust 完整构建、数据库迁移及 systemd/Nginx 实际启动成功。正式使用前应在新的 Ubuntu 26.04 测试实例执行首次安装、重复安装、更新和重启验证。

参考：[Ubuntu 软件包](https://packages.ubuntu.com/resolute/)、[SQLx CLI](https://github.com/launchbadge/sqlx/blob/v0.8.6/sqlx-cli/README.md)。
