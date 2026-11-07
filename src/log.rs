use napi_derive::napi;

#[napi]
pub mod log {
    #[napi]
    pub fn init_logger(app_data: String,filename: String,append: bool) -> String {
        logrs::init_logger(app_data,filename,append)
    }

    #[napi]
    pub fn test_panic() {
        panic!("This is a test panic");
    }
}