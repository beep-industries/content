use utoipa::OpenApi;

use crate::healthcheck::handlers::__path_get_healthcheck_handler;

use crate::storage::handlers::{
    post_object::__path_post_sign_url_handler, put_object::__path_put_object_handler,
};

#[derive(OpenApi)]
#[openapi(
    info(title = "Beep Content API",),
    paths(get_healthcheck_handler, put_object_handler, post_sign_url_handler)
)]
pub struct ApiDoc;
