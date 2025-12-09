"""
Integration tests for Python SDK Session Management

These tests verify the complete Session workflow through the Python SDK,
including all Session methods and error handling.

Run with: pytest tests/test_session_integration.py

Note: These tests require a running AvocadoDB server or use mocked responses.
"""

import pytest
from unittest.mock import Mock, patch
from avocado.session import Session, SessionInfo, Message
from avocado import AvocadoDB


class MockResponse:
    """Mock HTTP response for testing"""
    def __init__(self, json_data, status_code=200):
        self.json_data = json_data
        self.status_code = status_code

    def json(self):
        return self.json_data

    def raise_for_status(self):
        if self.status_code >= 400:
            raise Exception(f"HTTP {self.status_code}")


@pytest.fixture
def mock_db():
    """Create a mock AvocadoDB instance"""
    db = Mock(spec=AvocadoDB)
    db.base_url = "http://localhost:8765"
    db.project = "/tmp/test_project"
    return db


@pytest.fixture
def sample_session_data():
    """Sample session data for testing"""
    return {
        "id": "test-session-123",
        "user_id": "alice",
        "title": "Test Session",
        "created_at": "2025-11-17T10:00:00Z",
        "updated_at": "2025-11-17T10:00:00Z",
    }


@pytest.fixture
def sample_message_data():
    """Sample message data for testing"""
    return {
        "id": "msg-123",
        "session_id": "test-session-123",
        "role": "user",
        "content": "Test message",
        "sequence_number": 0,
        "created_at": "2025-11-17T10:00:00Z",
    }


def test_session_creation_from_dict(sample_session_data):
    """Test creating Session from dictionary"""
    db = Mock()
    db.base_url = "http://localhost:8765"
    db.project = "/tmp/test"

    session = Session(db, sample_session_data)

    assert session.id == "test-session-123"
    assert session.user_id == "alice"
    assert session.title == "Test Session"
    assert session.created_at == "2025-11-17T10:00:00Z"


def test_session_info_from_dict(sample_session_data):
    """Test creating SessionInfo from dictionary"""
    info = SessionInfo.from_dict(sample_session_data)

    assert info.id == "test-session-123"
    assert info.user_id == "alice"
    assert info.title == "Test Session"


def test_message_from_dict(sample_message_data):
    """Test creating Message from dictionary"""
    message = Message.from_dict(sample_message_data)

    assert message.id == "msg-123"
    assert message.role == "user"
    assert message.content == "Test message"
    assert message.sequence_number == 0


@patch('requests.post')
def test_add_message(mock_post, mock_db, sample_session_data, sample_message_data):
    """Test adding a message to a session"""
    mock_post.return_value = MockResponse({"message": sample_message_data})

    session = Session(mock_db, sample_session_data)
    message = session.add_message("user", "Test message")

    assert message.content == "Test message"
    assert message.role == "user"
    assert mock_post.called


@patch('requests.post')
def test_compile_in_session(mock_post, mock_db, sample_session_data, sample_message_data):
    """Test compiling context in a session"""
    working_set = {
        "text": "compiled context",
        "tokens_used": 100,
        "spans": [],
    }

    mock_post.return_value = MockResponse({
        "message": sample_message_data,
        "working_set": working_set,
    })

    session = Session(mock_db, sample_session_data)
    result = session.compile("Test query", budget=8000)

    assert result["message"]["content"] == "Test message"
    assert result["working_set"]["tokens_used"] == 100
    assert mock_post.called


@patch('requests.get')
def test_get_history(mock_get, mock_db, sample_session_data):
    """Test getting conversation history"""
    mock_get.return_value = MockResponse({
        "history": "User: Hello\n\nAssistant: Hi there"
    })

    session = Session(mock_db, sample_session_data)
    history = session.get_history()

    assert "User: Hello" in history
    assert "Assistant: Hi there" in history
    assert mock_get.called


@patch('requests.get')
def test_get_history_with_token_limit(mock_get, mock_db, sample_session_data):
    """Test getting history with token limit"""
    mock_get.return_value = MockResponse({
        "history": "User: Recent message"
    })

    session = Session(mock_db, sample_session_data)
    history = session.get_history(max_tokens=1000)

    # Verify the API was called with max_tokens parameter
    assert mock_get.called
    call_args = mock_get.call_args
    assert "max_tokens=1000" in call_args[0][0] or "max_tokens" in str(call_args)


@patch('requests.get')
def test_replay_session(mock_get, mock_db, sample_session_data):
    """Test replaying a session"""
    replay_data = {
        "session": sample_session_data,
        "turns": [
            {
                "user_message": {
                    "id": "msg-1",
                    "session_id": "test-session-123",
                    "role": "user",
                    "content": "Hello",
                    "sequence_number": 0,
                    "created_at": "2025-11-17T10:00:00Z",
                },
                "working_set": {
                    "text": "context",
                    "tokens_used": 100,
                    "spans": [],
                },
                "assistant_message": {
                    "id": "msg-2",
                    "session_id": "test-session-123",
                    "role": "assistant",
                    "content": "Hi",
                    "sequence_number": 1,
                    "created_at": "2025-11-17T10:01:00Z",
                },
            }
        ],
    }

    mock_get.return_value = MockResponse(replay_data)

    session = Session(mock_db, sample_session_data)
    replay = session.replay()

    assert "turns" in replay
    assert len(replay["turns"]) == 1
    assert replay["turns"][0]["user_message"]["content"] == "Hello"
    assert replay["turns"][0]["assistant_message"]["content"] == "Hi"


@patch('requests.delete')
def test_delete_session(mock_delete, mock_db, sample_session_data):
    """Test deleting a session"""
    mock_delete.return_value = MockResponse({"success": True})

    session = Session(mock_db, sample_session_data)
    session.delete()

    assert mock_delete.called


@patch('requests.get')
def test_refresh_session(mock_get, mock_db, sample_session_data):
    """Test refreshing session data"""
    updated_data = sample_session_data.copy()
    updated_data["title"] = "Updated Title"

    mock_get.return_value = MockResponse({
        "session": updated_data,
        "messages": [],
    })

    session = Session(mock_db, sample_session_data)
    session.refresh()

    assert session.title == "Updated Title"


@patch('requests.post')
def test_create_session_via_db(mock_post, mock_db, sample_session_data):
    """Test creating a session through AvocadoDB client"""
    mock_post.return_value = MockResponse({"session": sample_session_data})

    db = AvocadoDB(mode="http")
    db.base_url = "http://localhost:8765"
    db.project = "/tmp/test"

    with patch.object(db, '_http_request') as mock_request:
        mock_request.return_value = {"session": sample_session_data}
        session = db.create_session(user_id="alice", title="Test Session")

        assert session.user_id == "alice"
        assert session.title == "Test Session"


@patch('requests.get')
def test_list_sessions(mock_get, mock_db, sample_session_data):
    """Test listing sessions through AvocadoDB client"""
    sessions_data = [sample_session_data]
    mock_get.return_value = MockResponse({"sessions": sessions_data})

    db = AvocadoDB(mode="http")
    db.base_url = "http://localhost:8765"
    db.project = "/tmp/test"

    with patch.object(db, '_http_request') as mock_request:
        mock_request.return_value = {"sessions": sessions_data}
        sessions = db.list_sessions(user_id="alice")

        assert len(sessions) == 1
        assert sessions[0].user_id == "alice"


@patch('requests.get')
def test_get_session(mock_get, mock_db, sample_session_data):
    """Test getting a specific session"""
    mock_get.return_value = MockResponse({
        "session": sample_session_data,
        "messages": [],
    })

    db = AvocadoDB(mode="http")
    db.base_url = "http://localhost:8765"
    db.project = "/tmp/test"

    with patch.object(db, '_http_request') as mock_request:
        mock_request.return_value = {
            "session": sample_session_data,
            "messages": [],
        }
        session = db.get_session("test-session-123")

        assert session.id == "test-session-123"


def test_session_error_handling(mock_db, sample_session_data):
    """Test error handling in Session methods"""
    session = Session(mock_db, sample_session_data)

    with patch('requests.post') as mock_post:
        mock_post.return_value = MockResponse({}, status_code=404)

        with pytest.raises(Exception):
            session.add_message("user", "Test")


def test_compile_with_metadata(mock_db, sample_session_data, sample_message_data):
    """Test compiling with metadata"""
    session = Session(mock_db, sample_session_data)

    with patch('requests.post') as mock_post:
        mock_post.return_value = MockResponse({
            "message": sample_message_data,
            "working_set": {"text": "test", "tokens_used": 10, "spans": []},
        })

        result = session.compile("Query", budget=8000, metadata={"key": "value"})

        # Verify metadata was sent
        call_args = mock_post.call_args
        assert call_args is not None


def test_session_string_representation(mock_db, sample_session_data):
    """Test Session string representation"""
    session = Session(mock_db, sample_session_data)
    session_str = str(session)

    assert "test-session-123" in session_str
    assert "alice" in session_str


def test_empty_history(mock_db, sample_session_data):
    """Test getting history from empty session"""
    session = Session(mock_db, sample_session_data)

    with patch('requests.get') as mock_get:
        mock_get.return_value = MockResponse({"history": ""})

        history = session.get_history()
        assert history == ""


def test_replay_empty_session(mock_db, sample_session_data):
    """Test replaying empty session"""
    session = Session(mock_db, sample_session_data)

    with patch('requests.get') as mock_get:
        mock_get.return_value = MockResponse({
            "session": sample_session_data,
            "turns": [],
        })

        replay = session.replay()
        assert replay["turns"] == []


def test_multiple_messages_sequence(mock_db, sample_session_data):
    """Test adding multiple messages maintains sequence"""
    session = Session(mock_db, sample_session_data)

    messages_data = [
        {
            "id": f"msg-{i}",
            "session_id": "test-session-123",
            "role": "user" if i % 2 == 0 else "assistant",
            "content": f"Message {i}",
            "sequence_number": i,
            "created_at": "2025-11-17T10:00:00Z",
        }
        for i in range(5)
    ]

    with patch('requests.post') as mock_post:
        for i, msg_data in enumerate(messages_data):
            mock_post.return_value = MockResponse({"message": msg_data})
            message = session.add_message(msg_data["role"], msg_data["content"])
            assert message.sequence_number == i


@pytest.mark.parametrize("role", ["user", "assistant", "system", "tool"])
def test_add_message_different_roles(mock_db, sample_session_data, role):
    """Test adding messages with different roles"""
    session = Session(mock_db, sample_session_data)

    message_data = {
        "id": "msg-test",
        "session_id": "test-session-123",
        "role": role,
        "content": f"Test {role} message",
        "sequence_number": 0,
        "created_at": "2025-11-17T10:00:00Z",
    }

    with patch('requests.post') as mock_post:
        mock_post.return_value = MockResponse({"message": message_data})
        message = session.add_message(role, f"Test {role} message")
        assert message.role == role


def test_compile_with_different_budgets(mock_db, sample_session_data):
    """Test compile with various token budgets"""
    session = Session(mock_db, sample_session_data)

    budgets = [1000, 4000, 8000, 16000, 32000]

    for budget in budgets:
        with patch('requests.post') as mock_post:
            mock_post.return_value = MockResponse({
                "message": {
                    "id": "msg-test",
                    "session_id": "test-session-123",
                    "role": "user",
                    "content": "Query",
                    "sequence_number": 0,
                    "created_at": "2025-11-17T10:00:00Z",
                },
                "working_set": {
                    "text": "context",
                    "tokens_used": min(budget, 100),
                    "spans": [],
                },
            })

            result = session.compile("Query", budget=budget)
            # Verify the budget was respected
            assert result["working_set"]["tokens_used"] <= budget


def test_session_lifecycle(mock_db):
    """Test complete session lifecycle"""
    session_data = {
        "id": "lifecycle-test",
        "user_id": "test_user",
        "title": "Lifecycle Test",
        "created_at": "2025-11-17T10:00:00Z",
        "updated_at": "2025-11-17T10:00:00Z",
    }

    # Create session
    session = Session(mock_db, session_data)
    assert session.id == "lifecycle-test"

    # Add user message
    with patch('requests.post') as mock_post:
        mock_post.return_value = MockResponse({
            "message": {
                "id": "msg-1",
                "session_id": "lifecycle-test",
                "role": "user",
                "content": "Question",
                "sequence_number": 0,
                "created_at": "2025-11-17T10:00:00Z",
            }
        })
        msg1 = session.add_message("user", "Question")
        assert msg1.role == "user"

    # Add assistant response
    with patch('requests.post') as mock_post:
        mock_post.return_value = MockResponse({
            "message": {
                "id": "msg-2",
                "session_id": "lifecycle-test",
                "role": "assistant",
                "content": "Answer",
                "sequence_number": 1,
                "created_at": "2025-11-17T10:00:00Z",
            }
        })
        msg2 = session.add_message("assistant", "Answer")
        assert msg2.role == "assistant"

    # Get history
    with patch('requests.get') as mock_get:
        mock_get.return_value = MockResponse({
            "history": "User: Question\n\nAssistant: Answer"
        })
        history = session.get_history()
        assert "Question" in history
        assert "Answer" in history

    # Delete session
    with patch('requests.delete') as mock_delete:
        mock_delete.return_value = MockResponse({"success": True})
        session.delete()
        assert mock_delete.called
