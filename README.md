# AI 图片画廊（Rust + Axum + Askama + PostgreSQL）

展示 ComfyUI 生成的 AI 图片，用户付费（微信支付）解锁提示词的网站。含管理后台。

## 功能

**用户端**
- 首页：最新作品、分类导航
- 列表页 `/images`：分类筛选 + 分页
- 详情页 `/images/{id}`：大图、分类、AI 模型名称、图片尺寸、价格；未支付时提示词隐藏，支付后自动解锁
- 支付页 `/pay/{order_no}`：微信 Native 扫码支付（二维码），支付成功后前端轮询自动跳转并显示提示词

**管理后台 `/admin`**（账号见 `.env` 的 ADMIN_USERNAME / ADMIN_PASSWORD）
- 概览：图片/分类/订单数量与累计收入
- 分类管理：增删改、排序
- 图片管理：上传（自动压缩 + 自动读取宽高）、编辑、上下架、删除
- 订单管理：订单号、金额、支付状态、支付时间（仅记录，不做退款/退货）

**支付**
- 微信支付 v3 Native（扫码）下单 + 回调解密（AES-256-GCM）
- `wechat_pay.enabled = false` 时进入**模拟支付模式**，方便本地开发调试
- 支付成功后通过 HMAC 签名 cookie（`pids`）记录用户已购图片，永久解锁提示词；订单只存储，不做退款

## 运行

```bash
cargo run            # 或 cargo build --release 后运行 target/release/aiimg.exe
```

访问 http://127.0.0.1:8000（由 `.env` 的 `PORT` 决定），后台 http://127.0.0.1:8000/admin

数据库连接、监听地址和端口均在项目根目录 **`.env`** 文件中配置（不入库 git）：

```env
DATABASE_URL=postgres://用户名:密码@127.0.0.1:5432/image_blog
HOST=127.0.0.1
PORT=8000
```

首次启动自动建表（categories / images / orders / sessions），上传图片存 `uploads/`。单张图片上限 20MB（`main.rs` 中 `DefaultBodyLimit` 与 `image_handler.rs` 中的校验保持一致），ComfyUI 生成的数 MB PNG 可正常上传。

**图片压缩**（`src/img_compress.rs`，上传时自动执行，无需配置）：
- 无透明通道的图转 JPEG（质量 85），实测 ComfyUI 5.1MB PNG → 约 545KB（省约 90%）
- 有透明通道的图保留 PNG，使用最高压缩级别，透明区域不受影响
- 长边超过 4096 的图等比缩小（如 5000×1000 → 4096×819）
- GIF 保持原样（避免丢动画），压缩失败自动回退为原图
- 压缩前后大小会打印在日志：`图片压缩: 5251 KB -> 545 KB (1440×3200)`

## PostgreSQL 准备

```sql
CREATE DATABASE image_blog;
```

连接串填入 `.env` 的 `DATABASE_URL` 即可；若想放行密码登录，用 `postgres://aiimg:密码@127.0.0.1:5432/image_blog` 形式。

## 微信支付接入步骤

1. 微信商户平台申请 Native 支付权限，下载商户 API 证书，把私钥 `apiclient_key.pem` 放到项目根目录
2. 在 `.env` 中填写：`WECHAT_PAY_MCHID`（商户号）、`WECHAT_PAY_APPID`、`WECHAT_PAY_MCH_SERIAL_NO`（证书序列号）、`WECHAT_PAY_APIV3_KEY`（APIv3 密钥 32 位）、`WECHAT_PAY_NOTIFY_URL`（必须为 https 公网可达地址，如 `https://your-domain.com/notify/wechatpay`）
3. 把 `WECHAT_PAY_ENABLED=true`，重启即可

回调地址 `/notify/wechatpay` 需要能被微信服务器访问，本地调试请使用内网穿透（如 natapp / frp）。

## 配置说明（.env）

所有配置只来自 `.env`（环境变量同效），不再使用 config.toml：

| 变量 | 说明 |
|---|---|
| DATABASE_URL | PostgreSQL 连接串（**必填**） |
| HOST / PORT | 监听地址端口（可选，默认 127.0.0.1:8000） |
| ADMIN_USERNAME / ADMIN_PASSWORD | 后台登录账号（可选，默认 admin / admin123，上线务必修改） |
| SECRET | 已购 cookie 签名密钥（可选但上线务必改为随机长字符串） |
| WECHAT_PAY_ENABLED | false=模拟支付（开发，默认），true=真实微信支付 |
| WECHAT_PAY_MCHID / APPID / MCH_SERIAL_NO / MCH_PRIVATE_KEY_PATH / APIV3_KEY / NOTIFY_URL | 微信支付参数（启用真实支付时必填） |

## 技术栈

axum 0.7 · askama 0.12 模板 · sqlx 0.8 (PostgreSQL 连接池) · tower-http 静态服务 · rsa(SHA256withRSA 签名) · aes-gcm(回调解密) · qrcode(二维码生成) · imagesize(自动读取尺寸)

## 目录结构

```
src/main.rs              启动入口，组装路由
src/config.rs            配置加载（全部来自 .env / 环境变量）
src/db.rs                连接池与自动建表
src/auth.rs              cookie 会话与 HMAC 签名（公共）
src/pay.rs               微信支付 Native 下单 + 回调解密（公共）
src/img_compress.rs      上传图片压缩（转 JPEG / 保留透明 PNG / 超大图缩放）
src/state.rs             应用共享状态

src/service/             服务层：所有数据库查询/写入集中于此（不写 SQL 请勿改动其他层）
  mod.rs                   模块声明 + now_str
  cat_service.rs           分类：列表/带数量列表/增删改
  session_service.rs       管理员会话持久化（sessions 表，7 天有效，登录时自动清理过期会话）
  image_service.rs         图片：卡片/详情/分页/价格/编辑/增删改/上下架
  order_service.rs         订单：创建/状态流转/支付标记/列表/仪表盘统计

src/controller/          控制器层：HTTP 处理器，不直接写 SQL，统一调用 service
  mod.rs                   模块声明 + yuan
  site/                  用户端
    router.rs              路由定义
    index_handler.rs       首页/列表/详情/下单/支付/微信回调
  admin/                 管理后台
    mod.rs                 模块声明 + 登录守卫 guard
    model.rs               视图数据结构
    router.rs              路由定义
    auth_handler.rs        登录/登出
    index_handler.rs       概览仪表盘
    cat_handler.rs         分类增删改
    image_handler.rs       图片上传/编辑/上下架/删除
    order_handler.rs       订单列表

templates/site/          用户端模板    static/site/style.css    用户端样式
templates/admin/         后台模板      static/admin/admin.css   后台样式
```

main.rs 通过 `controller::site::router()` 与 `nest("/admin", controller::admin::router())` 组装路由。新增功能：数据库操作放 `src/service/`，页面逻辑放 `src/controller/` 对应的 handler 文件。

