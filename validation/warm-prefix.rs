use sysknife_brain::planning_tools::{get_state, preferences, propose_plan, query_tools, refuse};
use sysknife_brain::prompt::build_system_prompt;
use sysknife_types::{DistroHint, DISTRO_FAMILY_DEBIAN};

fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn main() {
    let hint = DistroHint {
        id: "ubuntu".into(),
        family: DISTRO_FAMILY_DEBIAN,
        version: Some("Ubuntu 24.04".into()),
    };
    let system = build_system_prompt(None, Some(&hint));
    let mut tools = vec![get_state::get_state_tool_def()];
    tools.extend(query_tools::query_tools_for_distro(Some(&hint)));
    tools.push(preferences::remember_tool_def());
    tools.push(preferences::forget_tool_def());
    tools.push(propose_plan::propose_plan_tool_def(Some(&hint)));
    tools.push(refuse::refuse_tool_def());
    let serialized: Vec<String> = tools.iter().map(|tool| format!(
        "{{\"type\":\"function\",\"function\":{{\"name\":{},\"description\":{},\"parameters\":{}}}}}",
        quote(&tool.name), quote(&tool.description), tool.input_schema
    )).collect();
    // This is a one-token, neutral initialization request, never a story answer.
    // The actual stories still run the unmodified CLI/harness with 600 seconds.
    println!(
        "{{\"model\":\"sysknife-pr695\",\"stream\":false,\"keep_alive\":\"30m\",\"messages\":[{{\"role\":\"system\",\"content\":{}}},{{\"role\":\"user\",\"content\":\"Acknowledge initialization briefly.\"}}],\"tools\":[{}],\"options\":{{\"num_predict\":1}}}}",
        quote(&system), serialized.join(",")
    );
}
