from dongne_ai.llm import EchoLLM


def test_echo():
    assert EchoLLM().complete("sys", "안녕") == "안녕"
