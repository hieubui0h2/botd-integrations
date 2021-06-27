use fastly::{Request, Response};
use crate::constants::*;
use crate::result_item::{get_result_item, ResultItem};
use crate::web_utils::{extract_header_value, get_cookie_from_request};
use crate::config::Config;

struct BotDetectionResult {
    pub request_id: String,
    pub request_status: String,
    pub error_description: String,

    pub automation_tool: ResultItem,
    pub search_bot: ResultItem,
    pub vm: ResultItem,
    pub browser_spoofing: ResultItem,
}

fn bot_detect(req: &Request, config: &Config) -> BotDetectionResult {
    let mut result = BotDetectionResult {
        request_id: "".to_owned(),
        request_status: "".to_owned(),
        error_description: "".to_owned(),

        automation_tool: ResultItem { ..Default::default() },
        search_bot: ResultItem { ..Default::default() },
        vm: ResultItem { ..Default::default() },
        browser_spoofing: ResultItem{ ..Default::default() },
    };

    // Get botd request id from cookie header
    let cookie_element = get_cookie_from_request(req, COOKIE_NAME);
    if cookie_element.is_none() {
        log::error!("[bot_detect] path: {}, cookie element cannot be found", req.get_path());
        result.request_status = ERROR.to_owned();
        return result;
    }
    let request_id = cookie_element.unwrap();
    result.request_id = request_id.to_owned();
    log::debug!("[bot_detect] path: {}, request_id = {}", req.get_path(), request_id.to_owned());

    // Build request for bot detection
    let url = format!("{}{}", config.botd_url.to_owned(), BOTD_RESULT_PATH);

    let mut query_str: String = "header&token=".to_owned();
    query_str.push_str(&*config.botd_token);
    query_str.push_str("&id=");
    query_str.push_str(request_id.as_str());

    log::debug!("[bot_detect] path: {}, url: {}?{}", req.get_path(), url.to_owned(), query_str);
    let verify_request = Request::get(url.to_owned()).with_query_str(query_str.to_owned());

    // Send verify request
    let verify_response = verify_request.send(BOTD_BACKEND).unwrap();

    // Check status code
    if !verify_response.get_status().is_success() {
        log::error!("[bot_detect] path: {}, verify status code: {}, link: {}?{}",
                    req.get_path(),
                    verify_response.get_status(),
                    url,
                    query_str.to_owned());
        result.request_status = ERROR.to_owned();
        return result;
    }

    // Extract request status
    let request_status_option = extract_header_value(verify_response.get_header(REQUEST_STATUS_HEADER));
    if request_status_option.is_none() {
        log::error!("[bot_detect] path: {}, request status cannot be found", req.get_path());
        result.request_status = ERROR.to_owned();
        return result;
    }
    let request_status = request_status_option.unwrap();
    if !request_status.eq(PROCESSED) {
        log::warn!("[bot_detect] path: {}, request status is {}, but expected OK", req.get_path(), request_status);
        result.request_status = request_status;
        let error_option = extract_header_value(verify_response.get_header(ERROR_DESCRIPTION));
        if error_option.is_some() {
            result.error_description = error_option.unwrap()
        }
        return result;
    }
    result.request_status = PROCESSED.to_owned();

    // Extract bot detection status
    result.automation_tool = get_result_item(&verify_response, AUTO_TOOL_STATUS_HEADER.to_owned(), AUTO_TOOL_PROB_HEADER.to_owned(), AUTO_TOOL_TYPE_HEADER.to_owned());

    // Extract search bot detection status
    result.search_bot = get_result_item(&verify_response, SEARCH_BOT_STATUS_HEADER.to_owned(), SEARCH_BOT_PROB_HEADER.to_owned(), SEARCH_BOT_TYPE_HEADER.to_owned());

    // Extract vm detection status
    result.vm = get_result_item(&verify_response, VM_STATUS_HEADER.to_owned(), VM_PROB_HEADER.to_owned(), VM_TYPE_HEADER.to_owned());

    // Extract browser spoofing detection status
    result.browser_spoofing = get_result_item(&verify_response, BROWSER_SPOOFING_STATUS_HEADER.to_owned(), BROWSER_SPOOFING_PROB_HEADER.to_owned(), BROWSER_SPOOFING_TYPE_HEADER.to_owned());

    return result;
}

pub fn handle_request_with_bot_detect(mut req: Request, config: &Config) -> Response {
    let result = bot_detect(&req, &config);

    req = req.with_header(REQUEST_ID_HEADER, result.request_id.to_owned());
    req = req.with_header(REQUEST_STATUS_HEADER, result.request_status.to_owned());
    log::debug!("[handle_request_with_bot_detect] path: {}, {}: {}, {}: {}", req.get_path(), REQUEST_ID_HEADER, result.request_id.to_owned(),
                REQUEST_STATUS_HEADER, result.request_status.to_owned());

    if result.request_status.eq(PROCESSED) {
        // Set bot detection result to header
        req = req.with_header(AUTO_TOOL_STATUS_HEADER, result.automation_tool.status.as_str());
        if result.automation_tool.status.eq(PROCESSED) {
            req = req.with_header(AUTO_TOOL_PROB_HEADER, format!("{:.2}", result.automation_tool.probability));
            if result.automation_tool.kind.len() > 0 {
                req = req.with_header(AUTO_TOOL_TYPE_HEADER, result.automation_tool.kind.to_owned());
            }
            log::debug!("[handle_request_with_bot_detect] path: {}, {}: {}, {}: {}, {}: {}", req.get_path(), AUTO_TOOL_STATUS_HEADER,
                        result.automation_tool.status.as_str(), AUTO_TOOL_PROB_HEADER, result.automation_tool.probability,
                        AUTO_TOOL_TYPE_HEADER, result.automation_tool.kind.to_owned());
        } else {
            log::debug!("[handle_request_with_bot_detect] path: {}, {}: {}", req.get_path(), AUTO_TOOL_STATUS_HEADER, result.automation_tool.status.as_str());
        }

        // Set search bot detection result to header
        req = req.with_header(SEARCH_BOT_STATUS_HEADER, result.search_bot.status.as_str());
        if result.search_bot.status.eq(PROCESSED) {
            req = req.with_header(SEARCH_BOT_PROB_HEADER, format!("{:.2}", result.search_bot.probability));
            if result.search_bot.kind.len() > 0 {
                req = req.with_header(SEARCH_BOT_TYPE_HEADER, result.search_bot.kind.to_owned());
            }
            log::debug!("[handle_request_with_bot_detect] path: {}, {}: {}, {}: {}, {}: {}", req.get_path(), SEARCH_BOT_STATUS_HEADER,
                        result.search_bot.status.as_str(), SEARCH_BOT_PROB_HEADER, result.search_bot.probability,
                        SEARCH_BOT_TYPE_HEADER, result.search_bot.kind.to_owned());
        } else {
            log::debug!("[handle_request_with_bot_detect] path: {}, {}: {}", req.get_path(), SEARCH_BOT_STATUS_HEADER, result.search_bot.status.as_str());
        }

        // Set vm detection result to header
        req = req.with_header(VM_STATUS_HEADER, result.vm.status.as_str());
        if result.vm.status.eq(PROCESSED) {
            req = req.with_header(VM_PROB_HEADER, format!("{:.2}", result.vm.probability));
            if result.vm.kind.len() > 0 {
                req = req.with_header(VM_TYPE_HEADER, result.vm.kind.to_owned());
            }
            log::debug!("[handle_request_with_bot_detect] path: {}, {}: {}, {}: {}, {}: {}", req.get_path(), VM_STATUS_HEADER,
                        result.vm.status.as_str(), VM_PROB_HEADER, result.vm.probability,
                        VM_TYPE_HEADER, result.vm.kind.to_owned());
        } else {
            log::debug!("[handle_request_with_bot_detect] path: {}, {}: {}", req.get_path(), VM_STATUS_HEADER, result.vm.status.as_str());
        }

        // Set browser spoofing detection result to header
        req = req.with_header(BROWSER_SPOOFING_STATUS_HEADER, result.browser_spoofing.status.as_str());
        if result.browser_spoofing.status.eq(PROCESSED) {
            req = req.with_header(BROWSER_SPOOFING_PROB_HEADER, format!("{:.2}", result.browser_spoofing.probability));
            log::debug!("[handle_request_with_bot_detect] path: {}, {}: {}, {}: {}, {}: {}", req.get_path(), BROWSER_SPOOFING_STATUS_HEADER,
                        result.browser_spoofing.status.as_str(), BROWSER_SPOOFING_PROB_HEADER, result.browser_spoofing.probability,
                        BROWSER_SPOOFING_TYPE_HEADER, result.browser_spoofing.kind.to_owned());
        } else {
            log::debug!("[handle_request_with_bot_detect] path: {}, {}: {}", req.get_path(), BROWSER_SPOOFING_STATUS_HEADER, result.browser_spoofing.status.as_str());
        }
    }

    return req.send(APP_BACKEND).unwrap();
}