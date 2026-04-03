# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.input.humanizer — human-like mouse and typing simulation."""

import math
import random
from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.input.humanizer import (
    _bezier_point,
    _ease_in_out,
    _generate_trajectory,
    get_cursor_position,
    human_move,
    typing_delays,
)


# ---------------------------------------------------------------------------
# Pure function tests (no mocking needed)
# ---------------------------------------------------------------------------

class TestBezierPoint:
    """Tests for _bezier_point."""

    def test_at_t0_returns_p0(self):
        """At t=0, the Bezier curve should be at the first control point."""
        result = _bezier_point(0.0, (0, 0), (100, 100), (200, 200), (300, 300))
        assert result == (0, 0)

    def test_at_t1_returns_p3(self):
        """At t=1, the Bezier curve should be at the last control point."""
        result = _bezier_point(1.0, (0, 0), (100, 100), (200, 200), (300, 300))
        assert result == (300, 300)

    def test_midpoint_on_straight_line(self):
        """For collinear control points, midpoint should be near the center."""
        result = _bezier_point(0.5, (0, 0), (100, 0), (200, 0), (300, 0))
        assert result == (150, 0)

    def test_returns_integer_tuple(self):
        """Result should always be a tuple of two ints."""
        result = _bezier_point(0.3, (10, 20), (50, 80), (90, 30), (150, 100))
        assert isinstance(result, tuple)
        assert len(result) == 2
        assert isinstance(result[0], int)
        assert isinstance(result[1], int)


class TestEaseInOut:
    """Tests for _ease_in_out."""

    def test_at_zero(self):
        """ease(0) should be 0."""
        assert _ease_in_out(0.0) == pytest.approx(0.0)

    def test_at_one(self):
        """ease(1) should be 1."""
        assert _ease_in_out(1.0) == pytest.approx(1.0)

    def test_at_half(self):
        """ease(0.5) should be 0.5 (symmetry point)."""
        assert _ease_in_out(0.5) == pytest.approx(0.5)

    def test_monotonically_increasing(self):
        """The easing function should be monotonically increasing."""
        values = [_ease_in_out(t / 100.0) for t in range(101)]
        for i in range(1, len(values)):
            assert values[i] >= values[i - 1]

    def test_slow_start(self):
        """The first quarter should cover less than 25% of the range."""
        assert _ease_in_out(0.25) < 0.25

    def test_slow_end(self):
        """The last quarter should cover less than 25% of the range."""
        assert _ease_in_out(0.75) > 0.75


class TestGenerateTrajectory:
    """Tests for _generate_trajectory."""

    def test_starts_at_origin(self):
        """Trajectory should start at the from-point."""
        random.seed(42)
        traj = _generate_trajectory(100, 200, 400, 500)
        assert traj[0] == (100, 200)

    def test_ends_near_target(self):
        """Trajectory should end near the target (within jitter of +/-2)."""
        random.seed(42)
        traj = _generate_trajectory(100, 200, 400, 500)
        last = traj[-1]
        assert abs(last[0] - 400) <= 2
        assert abs(last[1] - 500) <= 2

    def test_minimum_steps(self):
        """Even for very short distances, trajectory has at least a few points."""
        random.seed(42)
        traj = _generate_trajectory(10, 10, 11, 11)
        # min steps is 15, so at least 15 + 1 points before dedup
        assert len(traj) >= 2

    def test_longer_distance_more_steps(self):
        """Longer distances should produce more trajectory points."""
        random.seed(42)
        short_traj = _generate_trajectory(0, 0, 10, 10)
        random.seed(42)
        long_traj = _generate_trajectory(0, 0, 1000, 1000)
        assert len(long_traj) >= len(short_traj)

    def test_no_consecutive_duplicates(self):
        """Trajectory should have no consecutive duplicate points."""
        random.seed(42)
        traj = _generate_trajectory(0, 0, 500, 500)
        for i in range(1, len(traj)):
            assert traj[i] != traj[i - 1]

    def test_all_points_are_int_tuples(self):
        """Every point in the trajectory should be a tuple of ints."""
        random.seed(42)
        traj = _generate_trajectory(0, 0, 300, 300)
        for point in traj:
            assert isinstance(point, tuple)
            assert isinstance(point[0], int)
            assert isinstance(point[1], int)


class TestTypingDelays:
    """Tests for typing_delays."""

    def test_length_matches_text(self):
        """Should return one delay per character."""
        delays = typing_delays("hello")
        assert len(delays) == 5

    def test_empty_string(self):
        """Empty string returns empty list."""
        assert typing_delays("") == []

    def test_all_delays_positive(self):
        """All delays should be positive (minimum 10ms / 1000 = 0.01s)."""
        random.seed(42)
        delays = typing_delays("some text with punctuation! and spaces.")
        for d in delays:
            assert d >= 0.01

    def test_spaces_produce_longer_delays(self):
        """On average, space delays should be longer than letter delays."""
        random.seed(42)
        # Generate many samples to reduce randomness
        space_delays = []
        letter_delays = []
        for _ in range(500):
            ds = typing_delays("a b", base_delay_ms=50)
            letter_delays.append(ds[0])  # 'a'
            space_delays.append(ds[1])   # ' '
            letter_delays.append(ds[2])  # 'b'

        avg_space = sum(space_delays) / len(space_delays)
        avg_letter = sum(letter_delays) / len(letter_delays)
        assert avg_space > avg_letter

    def test_punctuation_produces_longer_delays(self):
        """On average, punctuation delays should be longer than letter delays."""
        random.seed(42)
        punct_delays = []
        letter_delays = []
        for _ in range(500):
            ds = typing_delays("a.", base_delay_ms=50)
            letter_delays.append(ds[0])  # 'a'
            punct_delays.append(ds[1])   # '.'

        avg_punct = sum(punct_delays) / len(punct_delays)
        avg_letter = sum(letter_delays) / len(letter_delays)
        assert avg_punct > avg_letter

    def test_custom_base_delay(self):
        """Custom base_delay_ms should scale all delays."""
        random.seed(42)
        slow = typing_delays("abc", base_delay_ms=200)
        random.seed(42)
        fast = typing_delays("abc", base_delay_ms=50)
        # Slow delays should be roughly 4x the fast ones
        for s, f in zip(slow, fast):
            assert s > f

    def test_all_punctuation_chars_handled(self):
        """All punctuation characters should produce delays."""
        random.seed(42)
        delays = typing_delays(".,;:!?\n")
        assert len(delays) == 7
        for d in delays:
            assert d >= 0.01


# ---------------------------------------------------------------------------
# Async function tests (mock ghostdesk._cmd.run)
# ---------------------------------------------------------------------------

class TestHumanMove:
    """Tests for human_move."""

    async def test_calls_xdotool_mousemove(self):
        """human_move should call xdotool mousemove for each trajectory point and final snap."""
        with patch("ghostdesk.input.humanizer.run", new_callable=AsyncMock) as mock_run, \
             patch("asyncio.sleep", new_callable=AsyncMock):
            mock_run.return_value = ""

            await human_move(0, 0, 100, 100)

            # Should have called run multiple times (trajectory + final snap)
            assert mock_run.call_count >= 2

            # Last call should be the exact target (final snap)
            last_call = mock_run.call_args_list[-1]
            assert last_call[0][0] == ["xdotool", "mousemove", "100", "100"]

    async def test_first_trajectory_call_starts_at_origin(self):
        """First xdotool call should be at/near the starting point."""
        with patch("ghostdesk.input.humanizer.run", new_callable=AsyncMock) as mock_run, \
             patch("asyncio.sleep", new_callable=AsyncMock):
            random.seed(42)
            mock_run.return_value = ""

            await human_move(50, 60, 300, 400)

            # First call should move to the starting point
            first_call = mock_run.call_args_list[0]
            cmd = first_call[0][0]
            assert cmd[0] == "xdotool"
            assert cmd[1] == "mousemove"


class TestGetCursorPosition:
    """Tests for get_cursor_position."""

    async def test_parses_xdotool_output(self):
        """get_cursor_position should parse xdotool getmouselocation output."""
        with patch("ghostdesk.input.humanizer.run", new_callable=AsyncMock) as mock_run:
            mock_run.return_value = "x:123 y:456 screen:0 window:789"

            x, y = await get_cursor_position()

            assert x == 123
            assert y == 456
            mock_run.assert_awaited_once_with(["xdotool", "getmouselocation"])

    async def test_parses_different_coordinates(self):
        """get_cursor_position works with various coordinate values."""
        with patch("ghostdesk.input.humanizer.run", new_callable=AsyncMock) as mock_run:
            mock_run.return_value = "x:0 y:0 screen:0 window:12345"

            x, y = await get_cursor_position()

            assert x == 0
            assert y == 0
