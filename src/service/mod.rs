//! 服务层：所有数据库查询与写入操作集中在此目录
//!
//! - `cat_service`    分类相关
//! - `image_service`  图片相关
//! - `order_service`  订单与统计相关
//!
//! 控制器层（src/controller）不直接写 SQL，一律调用本层函数。

pub mod cat_service;
pub mod image_service;
pub mod order_service;
pub mod session_service;

/// 当前时间字符串（与数据库 created_at / paid_at 格式一致）
pub(crate) fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
