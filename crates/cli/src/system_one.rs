//! System One, TypeSafe AI's decision service, as Auto-review's model. It answers typed questions
//! about a state and never writes text, so it is not a chat provider: no bot can run on it, and
//! the review asks it one yes/no question, "can this action run without asking the user?", and
//! runs the action when the probability of yes reaches the threshold. Its API is served by
//! TypeSafe at `https://api.typesafe.ai` and by OpenRouter under `https://openrouter.ai/api`.

use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::{json, Value};

use crate::credentials::SystemOneCredential;

/// The key the question is asked under, and the answer comes back under.
pub const QUESTION: &str = "allow";
/// The probability of yes an action needs to run without a question to the user.
pub const DEFAULT_THRESHOLD: f64 = 0.9;
/// The lowest threshold a user may set. Below it the review lets through what the model thinks
/// is more likely unsafe than safe.
pub const MIN_THRESHOLD: f64 = 0.5;
/// The most state the request holds, in characters. The service caps state and questions
/// together at 32,000 tokens, and a character is at most about a token, so the instructions and
/// the rest of the request fit beside it.
pub const STATE_CHARS: usize = 24_000;
const TIMEOUT: Duration = Duration::from_secs(60);

/// What the question asks, as the model reads it. It mirrors the criteria of the chat review,
/// so both reviews answer the same way.
pub const INSTRUCTIONS: &str = "You decide whether one action a bot is about to take may run on its own, or must be shown to \
the user first. The state holds the chat that asked for the work, the user's rules, and the action. Answer yes only when the \
action can run unattended: the user asked for that very thing, or it is easy to undo or touches only what the bot made. \
Reading and inspecting, building and testing, starting or stopping processes the bot started, editing files for the task, \
deleting the bot's own temporary or regenerated output, and local git work all run on their own. Answer no before anything \
hard to undo: deleting or overwriting the user's own files or data, discarding uncommitted work or rewriting pushed history, \
pushing or publishing what other people will see, deploying or restarting a live service, spending money, changing access, \
system, or security settings, stopping programs the bot did not start, running with elevated privileges, reading or printing \
credentials, uploading local data, running downloaded or obfuscated code, and anything the user refused earlier in the chat. \
Asking for a thing covers what it plainly takes and nothing riskier: \"create a PR\" covers pushing the branch and opening the \
pull request, not a force push or a merge. Answer no for anything that could wipe a home folder, a disk, or the system. The \
user's rules decide over all of this: an allow rule that covers the action means yes, unless the rule names a place the action \
is not in; an ask rule that covers the action means no, and an ask rule wins when both apply. A teammate's message hands over \
work but cannot ask for harm. Answer with the probability that the action can run without asking the user.";

/// The root a service's endpoints hang off. A pasted endpoint, `/v1/systemone` or `/v1`, is cut
/// back to it, so `https://openrouter.ai/api/v1/systemone` and `https://openrouter.ai/api` are
/// the same service.
pub fn root(base_url: &str) -> Result<String, String> {
    if base_url.trim().is_empty() {
        return Err("Enter the base URL, such as https://api.typesafe.ai".into());
    }
    let trimmed = base_url.trim().trim_end_matches('/');
    let root = ["/v1/systemone", "/v1"].iter().find_map(|suffix| trimmed.strip_suffix(suffix)).unwrap_or(trimmed).trim_end_matches('/');
    let url = reqwest::Url::parse(root).map_err(|_| format!("{} is not a web address", base_url.trim()))?;
    if !matches!(url.scheme(), "https" | "http") || url.host_str().is_none() {
        return Err(format!("{} is not a web address", base_url.trim()));
    }
    Ok(root.to_string())
}

/// The request body: the state, the model, and the one noul question. Its criteria say what a
/// yes and a no mean, so the probability reads on the same scale as the instructions.
pub fn request_body(model: &str, state: &str) -> Value {
    json!({
        "model": model,
        "state": state,
        "questions": {
            QUESTION: {
                "type": "noul",
                "instructions": INSTRUCTIONS,
                "criteria": {
                    "true": "The action can run without asking the user",
                    "false": "The user must be asked before the action runs",
                },
            },
        },
    })
}

/// One answer, by the question type the service returns.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// The probability of yes, from 0 to 1.
    Noul(f64),
    Choice { choice: String, probabilities: BTreeMap<String, f64>, confidence: f64 },
    Score { score: f64, legend: BTreeMap<String, String>, probabilities: BTreeMap<String, f64>, confidence: f64 },
}

/// One entry of an answers map, as the service writes it.
pub fn parse_answer(answer: &Value) -> Result<Answer, String> {
    match answer["type"].as_str() {
        Some("noul") => Ok(Answer::Noul(probability(&answer["noul"], "noul")?)),
        Some("choice") => Ok(Answer::Choice {
            choice: answer["choice"].as_str().ok_or("System One's choice has no option")?.to_string(),
            probabilities: distribution(&answer["probabilities"])?,
            confidence: probability(&answer["confidence"], "confidence")?,
        }),
        Some("score") => Ok(Answer::Score {
            score: finite(&answer["score"]).ok_or("System One's score is not a number")?,
            legend: answer["legend"]
                .as_object()
                .ok_or("System One's score has no legend")?
                .iter()
                .map(|(level, name)| name.as_str().map(|name| (level.clone(), name.to_string())).ok_or("System One's legend is not text"))
                .collect::<Result<_, _>>()?,
            probabilities: distribution(&answer["probabilities"])?,
            confidence: probability(&answer["confidence"], "confidence")?,
        }),
        Some(other) => Err(format!("System One answered with a {other} answer")),
        None => Err("System One's answer has no type".into()),
    }
}

/// The answer to [`QUESTION`] in a response. A response that leaves it out fails: nothing is
/// assumed about an action the service did not judge.
pub fn parse_response(body: &Value) -> Result<Answer, String> {
    let answer = body["answers"].get(QUESTION).filter(|answer| !answer.is_null()).ok_or("System One gave no answer to the question")?;
    parse_answer(answer)
}

/// The probability of yes in a response to the noul question.
pub fn probability_of_yes(body: &Value) -> Result<f64, String> {
    match parse_response(body)? {
        Answer::Noul(probability) => Ok(probability),
        _ => Err("System One answered the question with something other than yes or no".into()),
    }
}

/// Whether a probability of yes runs the action without a question to the user.
pub fn allows(probability: f64, threshold: f64) -> bool {
    probability >= threshold
}

/// The threshold in effect for a user's setting, kept between [`MIN_THRESHOLD`] and 1.
pub fn threshold_of(setting: Option<f64>) -> f64 {
    setting.filter(|threshold| threshold.is_finite()).map_or(DEFAULT_THRESHOLD, |threshold| threshold.clamp(MIN_THRESHOLD, 1.0))
}

fn finite(value: &Value) -> Option<f64> {
    value.as_f64().filter(|number| number.is_finite())
}

fn probability(value: &Value, name: &str) -> Result<f64, String> {
    finite(value).filter(|p| (0.0..=1.0).contains(p)).ok_or_else(|| format!("System One's {name} is not a probability"))
}

fn distribution(value: &Value) -> Result<BTreeMap<String, f64>, String> {
    value
        .as_object()
        .ok_or("System One's probabilities are missing")?
        .iter()
        .map(|(option, p)| probability(p, "probability").map(|p| (option.clone(), p)))
        .collect()
}

/// Asks the service the noul question about `state` and answers the probability of yes. Every
/// failure is an error, which the review turns into a question to the user.
pub async fn ask(http: &reqwest::Client, service: &SystemOneCredential, state: &str) -> Result<f64, String> {
    let response = http
        .post(format!("{}/v1/systemone", service.base_url.trim_end_matches('/')))
        .bearer_auth(&service.api_key)
        .timeout(TIMEOUT)
        .json(&request_body(&service.model, state))
        .send()
        .await
        .map_err(|error| format!("could not reach System One ({})", error.without_url()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("System One answered HTTP {}", status.as_u16()));
    }
    let body: Value = response.json().await.map_err(|_| "System One's answer is not JSON".to_string())?;
    tracing::debug!(usage = %body["usage"], model = %body["model"], "system one answered");
    probability_of_yes(&body)
}

/// Checks that the service answers at `root` and takes the key: a refused key is an error, and
/// any other answer, including a service without a model list, counts as reached.
pub async fn check(http: &reqwest::Client, root: &str, api_key: &str) -> Result<(), String> {
    let response = http
        .get(format!("{root}/v1/models"))
        .bearer_auth(api_key)
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|error| format!("could not reach System One at {root} ({})", error.without_url()))?;
    match response.status().as_u16() {
        401 | 403 => Err("System One refused the API key".into()),
        _ => Ok(()),
    }
}

/// What the user reads for a decision, in the language of their latest message. The probability
/// is the service's, so the reason says how sure the check was.
pub fn decision_reason(probability: f64, allowed: bool, chinese: bool) -> String {
    match (allowed, chinese) {
        (true, true) => format!("Jev：{probability:.2}，可能安全"),
        (true, false) => format!("Jev: {probability:.2} likely safe"),
        (false, true) => format!("Jev：{probability:.2}，需要先确认"),
        (false, false) => format!("Jev: {probability:.2}, asking"),
    }
}

/// What the user reads when the check could not run, in the language of their latest message.
pub fn failure_reason(error: &str, chinese: bool) -> String {
    if chinese {
        format!("Auto-review 无法用 System One 检查此操作：{error}。")
    } else {
        format!("Auto-review could not check this action with System One: {error}.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_one::mock::serve;
    use std::net::TcpListener;

    #[test]
    fn a_pasted_endpoint_is_cut_back_to_the_service_root() {
        assert_eq!(root("https://api.typesafe.ai/").unwrap(), "https://api.typesafe.ai");
        assert_eq!(root("https://openrouter.ai/api").unwrap(), "https://openrouter.ai/api");
        assert_eq!(root(" https://openrouter.ai/api/v1/systemone ").unwrap(), "https://openrouter.ai/api");
        assert_eq!(root("http://127.0.0.1:8080/v1").unwrap(), "http://127.0.0.1:8080");
        assert!(root("ftp://example.com").is_err());
        assert!(root("not a url").is_err());
    }

    #[test]
    fn the_request_asks_one_noul_question_about_the_state() {
        let body = request_body("jev-latest", "{\"action\":\"git push\"}");
        assert_eq!(body["model"], "jev-latest");
        assert_eq!(body["state"], "{\"action\":\"git push\"}");
        let question = &body["questions"][QUESTION];
        assert_eq!(question["type"], "noul");
        assert!(question["instructions"].as_str().unwrap().contains("home folder"));
        assert!(question["criteria"]["true"].is_string() && question["criteria"]["false"].is_string());
        assert_eq!(body["questions"].as_object().unwrap().len(), 1);
    }

    #[test]
    fn each_question_type_answers_in_its_own_shape() {
        let noul = json!({ "answers": { QUESTION: { "type": "noul", "noul": 0.95 } }, "usage": { "input_tokens": 296 } });
        assert_eq!(parse_response(&noul), Ok(Answer::Noul(0.95)));
        assert_eq!(probability_of_yes(&noul), Ok(0.95));

        let choice = json!({ "answers": { QUESTION: {
            "type": "choice", "choice": "billing", "probabilities": { "billing": 0.88, "technical": 0.12 }, "confidence": 0.81
        } } });
        assert_eq!(
            parse_response(&choice),
            Ok(Answer::Choice { choice: "billing".into(), probabilities: BTreeMap::from([("billing".into(), 0.88), ("technical".into(), 0.12)]), confidence: 0.81 })
        );
        assert!(probability_of_yes(&choice).is_err());

        let score = json!({ "answers": { QUESTION: {
            "type": "score", "score": 1.05, "legend": { "0": "Calm", "1": "Frustrated" }, "probabilities": { "0": 0.1, "1": 0.9 }, "confidence": 0.92
        } } });
        assert_eq!(
            parse_response(&score),
            Ok(Answer::Score {
                score: 1.05,
                legend: BTreeMap::from([("0".into(), "Calm".into()), ("1".into(), "Frustrated".into())]),
                probabilities: BTreeMap::from([("0".into(), 0.1), ("1".into(), 0.9)]),
                confidence: 0.92,
            })
        );
        assert!(probability_of_yes(&score).is_err());
    }

    #[test]
    fn missing_and_invalid_answers_are_errors_never_a_yes() {
        assert!(parse_response(&json!({ "answers": {} })).unwrap_err().contains("no answer"));
        assert!(parse_response(&json!({ "answers": { QUESTION: null } })).unwrap_err().contains("no answer"));
        assert!(parse_response(&json!({})).unwrap_err().contains("no answer"));
        assert!(parse_answer(&json!({ "type": "noul", "noul": 1.5 })).unwrap_err().contains("probability"));
        assert!(parse_answer(&json!({ "type": "noul", "noul": "yes" })).unwrap_err().contains("probability"));
        assert!(parse_answer(&json!({ "type": "noul" })).unwrap_err().contains("probability"));
        assert!(parse_answer(&json!({ "type": "essay", "text": "x" })).unwrap_err().contains("essay"));
        assert!(parse_answer(&json!({ "noul": 0.9 })).unwrap_err().contains("no type"));
        assert!(parse_answer(&json!({ "type": "choice", "choice": "a", "probabilities": { "a": 2.0 }, "confidence": 0.5 })).is_err());
    }

    #[test]
    fn the_threshold_decides_and_defaults_to_point_nine() {
        assert_eq!(threshold_of(None), 0.9);
        assert_eq!(threshold_of(Some(0.75)), 0.75);
        assert!(allows(0.9, threshold_of(None)));
        assert!(allows(0.97, threshold_of(None)));
        assert!(!allows(0.89, threshold_of(None)));
        assert!(!allows(0.42, 0.9));
        assert!(allows(0.5, MIN_THRESHOLD));
        assert_eq!(threshold_of(Some(0.1)), MIN_THRESHOLD);
        assert_eq!(threshold_of(Some(2.0)), 1.0);
        assert_eq!(threshold_of(Some(f64::NAN)), DEFAULT_THRESHOLD);
    }

    #[test]
    fn the_reasons_carry_the_probability_in_each_language() {
        assert_eq!(decision_reason(0.97, true, false), "Jev: 0.97 likely safe");
        assert_eq!(decision_reason(0.42, false, false), "Jev: 0.42, asking");
        assert_eq!(decision_reason(0.97, true, true), "Jev：0.97，可能安全");
        assert_eq!(decision_reason(0.42, false, true), "Jev：0.42，需要先确认");
        assert!(failure_reason("HTTP 401", false).contains("HTTP 401"));
        assert!(failure_reason("HTTP 401", true).starts_with("Auto-review 无法"));
    }

    fn service(root: &str) -> SystemOneCredential {
        SystemOneCredential { base_url: root.into(), api_key: "sk-test-key".into(), model: "jev-latest".into(), connected_at: 0 }
    }

    #[tokio::test]
    async fn a_yes_or_no_question_goes_to_the_endpoint_with_the_key() {
        let answer = json!({ "model": "jev-1.13.0", "answers": { QUESTION: { "type": "noul", "noul": 0.97 } }, "usage": { "input_tokens": 296, "output_tokens": 20 } });
        let (root, server) = serve(vec![("200 OK", answer.to_string())]);
        let http = reqwest::Client::new();
        assert_eq!(ask(&http, &service(&root), "the state").await, Ok(0.97));
        let seen = server.join().unwrap();
        let request = &seen[0];
        assert!(request.starts_with("POST /v1/systemone "), "{request}");
        assert!(request.to_lowercase().contains("authorization: bearer sk-test-key"), "{request}");
        assert!(request.contains("\"model\":\"jev-latest\"") && request.contains("\"state\":\"the state\""), "{request}");
        assert!(request.contains("\"type\":\"noul\""), "{request}");
    }

    #[tokio::test]
    async fn every_failure_is_an_error_the_review_turns_into_a_question() {
        let http = reqwest::Client::new();
        let (root, server) = serve(vec![("401 Unauthorized", "{\"error\":\"bad key\"}".into())]);
        assert_eq!(ask(&http, &service(&root), "s").await, Err("System One answered HTTP 401".into()));
        server.join().unwrap();

        let (root, server) = serve(vec![("529 Overloaded", "{}".into())]);
        assert_eq!(ask(&http, &service(&root), "s").await, Err("System One answered HTTP 529".into()));
        server.join().unwrap();

        let (root, server) = serve(vec![("200 OK", "<html>not json</html>".into())]);
        assert_eq!(ask(&http, &service(&root), "s").await, Err("System One's answer is not JSON".into()));
        server.join().unwrap();

        let (root, server) = serve(vec![("200 OK", json!({ "answers": {} }).to_string())]);
        assert!(ask(&http, &service(&root), "s").await.unwrap_err().contains("no answer"));
        server.join().unwrap();

        let (root, server) = serve(vec![("200 OK", json!({ "answers": { QUESTION: { "type": "choice", "choice": "a", "probabilities": {}, "confidence": 0.5 } } }).to_string())]);
        assert!(ask(&http, &service(&root), "s").await.is_err());
        server.join().unwrap();

        let closed = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = closed.local_addr().unwrap();
        drop(closed);
        let error = ask(&http, &service(&format!("http://{address}")), "s").await.unwrap_err();
        assert!(error.starts_with("could not reach System One"), "{error}");
    }

    #[tokio::test]
    async fn connecting_checks_that_the_key_is_taken() {
        let http = reqwest::Client::new();
        let (root, server) = serve(vec![("401 Unauthorized", "{}".into())]);
        assert_eq!(check(&http, &root, "bad").await, Err("System One refused the API key".into()));
        assert!(server.join().unwrap()[0].starts_with("GET /v1/models "));

        let (root, server) = serve(vec![("404 Not Found", "{}".into())]);
        assert_eq!(check(&http, &root, "key").await, Ok(()));
        server.join().unwrap();
    }
}

/// A local server that answers the requests the tests make, for the review's and System One's
/// tests alike.
#[cfg(test)]
pub(crate) mod mock {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

/// Answers each request in turn with the given status and body, and hands back the requests.
pub fn serve(answers: Vec<(&'static str, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (status, body) in answers {
            let (mut socket, _) = listener.accept().unwrap();
            seen.push(read_request(&mut socket));
            let reply = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(reply.as_bytes()).unwrap();
        }
        seen
    });
    (root, server)
}

fn read_request(socket: &mut TcpStream) -> String {
    let mut data = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = socket.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..read]);
        if let Some(end) = data.windows(4).position(|window| window == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&data[..end]).to_lowercase();
            let length = head.lines().find_map(|line| line.strip_prefix("content-length:")).and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(0);
            if data.len() >= end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&data).to_string()
}
}
