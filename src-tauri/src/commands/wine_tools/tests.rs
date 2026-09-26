use super::*;

#[test]
fn tool_arg_accepts_all_supported_builtin_tools() {
    for tool in BUILTIN_WINE_TOOLS {
        assert_eq!(tool_arg(tool).unwrap(), *tool);
    }
}

#[test]
fn tool_arg_rejects_unknown_tools_with_the_original_name() {
    match tool_arg("notepad") {
        Err(AppError::UnknownWineTool { tool }) => assert_eq!(tool, "notepad"),
        other => panic!("unexpected result: {other:?}"),
    }
}
