use actix_web::{HttpResponse, HttpResponseBuilder};

pub trait JsonResponse {
    fn json_response(
        &mut self,
        data: Option<serde_json::Value>,
        msg: Option<String>,
    ) -> HttpResponse;

    fn json_error(&mut self, msg: String) -> HttpResponse {
        self.json_response(None, Some(msg))
    }
}

impl JsonResponse for HttpResponseBuilder {
    fn json_response(
        &mut self,
        data: Option<serde_json::Value>,
        msg: Option<String>,
    ) -> HttpResponse {
        self.json(serde_json::json!({
          "msg":msg,
          "data":data,
        }))
    }
}
