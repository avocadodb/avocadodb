"""
Session management for AvocadoDB.

This module provides a high-level Session class for managing conversation
sessions, including message management, context compilation, and session replay.
"""

from typing import Optional, Dict, List, Any
from dataclasses import dataclass
import requests


@dataclass
class Message:
    """A message in a conversation session."""
    id: str
    session_id: str
    role: str
    content: str
    sequence_number: int
    created_at: str
    metadata: Optional[Dict[str, Any]] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Message':
        return cls(
            id=data['id'],
            session_id=data['session_id'],
            role=data['role'],
            content=data['content'],
            sequence_number=data['sequence_number'],
            created_at=data['created_at'],
            metadata=data.get('metadata')
        )


@dataclass
class SessionInfo:
    """Session metadata."""
    id: str
    user_id: Optional[str]
    title: Optional[str]
    created_at: str
    updated_at: str
    last_message_at: Optional[str] = None
    metadata: Optional[Dict[str, Any]] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'SessionInfo':
        return cls(
            id=data['id'],
            user_id=data.get('user_id'),
            title=data.get('title'),
            created_at=data['created_at'],
            updated_at=data['updated_at'],
            last_message_at=data.get('last_message_at'),
            metadata=data.get('metadata')
        )


class Session:
    """Represents a conversation session with AvocadoDB.

    A session tracks conversation history and provides methods for:
    - Adding user and assistant messages
    - Compiling context for queries
    - Retrieving conversation history
    - Replaying session for debugging

    Example:
        >>> from avocado import AvocadoDB
        >>> db = AvocadoDB()
        >>>
        >>> # Create a new session
        >>> session = db.create_session(user_id="alice", title="My Conversation")
        >>>
        >>> # Compile context and add user message
        >>> result = session.compile("What is Rust?")
        >>> print(result['working_set']['text'])
        >>>
        >>> # Add assistant response
        >>> session.add_message("assistant", "Rust is a systems programming language...")
        >>>
        >>> # Get conversation history
        >>> history = session.get_history()
        >>> print(history)
    """

    def __init__(self, client, session_or_data, session_info: Optional[SessionInfo] = None):
        """Initialize a Session.

        Args:
            client: AvocadoDB client instance
            session_or_data: Session ID (str) or dict with session fields
            session_info: Optional session metadata (ignored if dict is provided)
        """
        self.client = client
        # Allow constructing from dict for tests/convenience
        if isinstance(session_or_data, dict):
            info = SessionInfo.from_dict(session_or_data)
            self.id = info.id
            self.session_id = info.id
            self._info = info
            self.user_id = info.user_id
            self.title = info.title
            self.created_at = info.created_at
            self.updated_at = info.updated_at
        else:
            self.id = str(session_or_data)
            self.session_id = str(session_or_data)
            self._info = session_info
            if session_info is not None:
                self.user_id = session_info.user_id
                self.title = session_info.title
                self.created_at = session_info.created_at
                self.updated_at = session_info.updated_at

    @property
    def info(self) -> SessionInfo:
        """Get session information (lazily loaded)."""
        if self._info is None:
            self._info = self._fetch_info()
        return self._info

    def _fetch_info(self) -> SessionInfo:
        """Fetch session info from server."""
        base_url = getattr(self.client, "url", getattr(self.client, "base_url", None))
        project = getattr(self.client, "project_path", getattr(self.client, "project", "."))
        if base_url:
            response = requests.get(
                f"{base_url}/sessions/{self.session_id}",
                params={"project": project}
            )
            response.raise_for_status()
            data = response.json()
            return SessionInfo.from_dict(data['session'])
        else:
            # CLI mode doesn't support sessions yet
            raise NotImplementedError("Session management not available in CLI mode")

    def add_message(
        self,
        role: str,
        content: str,
        metadata: Optional[Dict[str, Any]] = None
    ) -> Message:
        """Add a message to the session.

        Args:
            role: Message role ('user', 'assistant', 'system', or 'tool')
            content: Message content
            metadata: Optional metadata (tool calls, citations, etc.)

        Returns:
            The created Message object

        Example:
            >>> session.add_message("assistant", "Rust is great!")
        """
        base_url = getattr(self.client, "url", getattr(self.client, "base_url", None))
        project = getattr(self.client, "project_path", getattr(self.client, "project", "."))
        if base_url:
            response = requests.post(
                f"{base_url}/sessions/{self.session_id}/messages",
                json={
                    "role": role,
                    "content": content,
                    "metadata": metadata,
                    "project": project
                }
            )
            response.raise_for_status()
            data = response.json()
            return Message.from_dict(data['message'])
        else:
            raise NotImplementedError("Session management not available in CLI mode")

    def compile(
        self,
        query: str,
        budget: int = 8000,
        **kwargs
    ) -> Dict[str, Any]:
        """Compile context for a query within this session.

        This automatically:
        1. Adds the query as a user message
        2. Compiles the context using the compiler
        3. Associates the working set with the session

        Args:
            query: The user's query
            budget: Token budget for compilation (default: 8000)
            **kwargs: Additional compiler configuration

        Returns:
            Dict with 'message' (Message object) and 'working_set' (WorkingSet)

        Example:
            >>> result = session.compile("How does authentication work?")
            >>> print(result['working_set']['text'])
            >>> print(result['message']['content'])
        """
        base_url = getattr(self.client, "url", getattr(self.client, "base_url", None))
        project = getattr(self.client, "project_path", getattr(self.client, "project", "."))
        if base_url:
            response = requests.post(
                f"{base_url}/sessions/{self.session_id}/compile",
                json={
                    "query": query,
                    "token_budget": budget,
                    "project": project,
                    # TODO: Support additional config from kwargs
                }
            )
            response.raise_for_status()
            data = response.json()

            return data
        else:
            raise NotImplementedError("Session management not available in CLI mode")

    def get_history(self, max_tokens: Optional[int] = None) -> str:
        """Get formatted conversation history.

        Returns the conversation history formatted as:
        ```
        User: <message>

        Assistant: <message>

        User: <message>
        ...
        ```

        Args:
            max_tokens: Optional token limit (keeps most recent messages)

        Returns:
            Formatted conversation history as a string

        Example:
            >>> history = session.get_history(max_tokens=1000)
            >>> print(history)
        """
        base_url = getattr(self.client, "url", getattr(self.client, "base_url", None))
        project = getattr(self.client, "project_path", getattr(self.client, "project", "."))
        if base_url:
            params = {"project": project}
            if max_tokens is not None:
                params["max_tokens"] = max_tokens

            response = requests.get(
                f"{base_url}/sessions/{self.session_id}/history",
                params=params
            )
            response.raise_for_status()
            data = response.json()
            return data['history']
        else:
            raise NotImplementedError("Session management not available in CLI mode")

    def replay(self) -> Dict[str, Any]:
        """Get full session replay for debugging.

        Returns session data with all turns (user message + working set + assistant response).
        Useful for debugging and understanding agent behavior.

        Returns:
            Dict with 'session' and 'turns' (list of conversation turns)

        Example:
            >>> replay = session.replay()
            >>> for turn in replay['turns']:
            ...     print(f"User: {turn['user_message']['content']}")
            ...     if turn['assistant_message']:
            ...         print(f"Assistant: {turn['assistant_message']['content']}")
        """
        base_url = getattr(self.client, "url", getattr(self.client, "base_url", None))
        project = getattr(self.client, "project_path", getattr(self.client, "project", "."))
        if base_url:
            response = requests.get(
                f"{base_url}/sessions/{self.session_id}/replay",
                params={"project": project}
            )
            response.raise_for_status()
            return response.json()
        else:
            raise NotImplementedError("Session management not available in CLI mode")

    def delete(self) -> bool:
        """Delete this session and all associated data.

        Returns:
            True if successful

        Example:
            >>> session.delete()
            True
        """
        base_url = getattr(self.client, "url", getattr(self.client, "base_url", None))
        project = getattr(self.client, "project_path", getattr(self.client, "project", "."))
        if base_url:
            response = requests.delete(
                f"{base_url}/sessions/{self.session_id}",
                params={"project": project}
            )
            response.raise_for_status()
            data = response.json()
            return data['success']
        else:
            raise NotImplementedError("Session management not available in CLI mode")

    def refresh(self) -> None:
        """Refresh this session's metadata from the server."""
        info = self._fetch_info()
        self._info = info
        # Sync common fields for convenience
        self.id = info.id
        self.session_id = info.id
        self.user_id = info.user_id
        self.title = info.title
        self.created_at = info.created_at
        self.updated_at = info.updated_at

    def get_messages(self, limit: Optional[int] = None) -> List[Message]:
        """Get all messages in this session.

        Args:
            limit: Optional limit on number of messages to return

        Returns:
            List of Message objects in chronological order

        Example:
            >>> messages = session.get_messages()
            >>> for msg in messages:
            ...     print(f"{msg.role}: {msg.content}")
        """
        if self.client.mode == "http":
            response = self.client.session.get(
                f"{self.client.url}/sessions/{self.session_id}",
                params={"project": self.client.project_path}
            )
            response.raise_for_status()
            data = response.json()
            messages = [Message.from_dict(m) for m in data['messages']]

            if limit is not None:
                messages = messages[:limit]

            return messages
        else:
            raise NotImplementedError("Session management not available in CLI mode")

    def __repr__(self) -> str:
        user = getattr(self, "user_id", None) or (self._info.user_id if self._info else None) or "N/A"
        return f"Session(id='{self.session_id}', user='{user}')"
