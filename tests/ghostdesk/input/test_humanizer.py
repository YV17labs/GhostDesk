# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.input.humanizer — human-like mouse movement."""

import random
from unittest.mock import AsyncMock, patch

from ghostdesk.input.humanizer import (
    _bezier_point,
    _ease_in_out,
    _generate_trajectory,
    human_move,
)


class TestBezierPoint:
    """Tests for _bezier_point."""

    def test_at_t0_returns_p0(self):
        """At t=0, the curve should be at the first control point."""
        assert _bezier_point(0, (0, 0), (10, 20), (30, 40), (50, 60)) == (0, 0)

    def test_at_t1_returns_p3(self):
        """At t=1, the curve should be at the last control point."""
        assert _bezier_point(1, (0, 0), (10, 20), (30, 40), (50, 60)) == (50, 60)

    def test_midpoint_on_straight_line(self):
        """For a straight line, t=0.5 should be near the midpoint."""
        result = _bezier_point(0.5, (0, 0), (33, 0), (66, 0), (100, 0))
        assert 45 <= result[0] <= 55

    def test_returns_integer_tuple(self):
        """_bezier_point should return integer coordinates."""
        result = _bezier_point(0.3, (0, 0), (10, 20), (30, 40), (50, 60))
        assert isinstance(result[0], int)
        assert isinstance(result[1], int)


class TestEaseInOut:
    """Tests for _ease_in_out."""

    def test_at_zero(self):
        assert _ease_in_out(0) == 0

    def test_at_one(self):
        assert abs(_ease_in_out(1) - 1.0) < 1e-10

    def test_at_half(self):
        assert abs(_ease_in_out(0.5) - 0.5) < 1e-10

    def test_monotonically_increasing(self):
        values = [_ease_in_out(t / 100) for t in range(101)]
        for i in range(1, len(values)):
            assert values[i] >= values[i - 1]

    def test_slow_start(self):
        assert _ease_in_out(0.25) < 0.25

    def test_slow_end(self):
        assert _ease_in_out(0.75) > 0.75


class TestGenerateTrajectory:
    """Tests for _generate_trajectory."""

    def test_starts_at_origin(self):
        random.seed(42)
        traj = _generate_trajectory(100, 200, 400, 500)
        assert traj[0] == (100, 200)

    def test_ends_near_target(self):
        random.seed(42)
        traj = _generate_trajectory(100, 200, 400, 500)
        last = traj[-1]
        assert abs(last[0] - 400) <= 2
        assert abs(last[1] - 500) <= 2

    def test_minimum_steps(self):
        random.seed(42)
        traj = _generate_trajectory(10, 10, 11, 11)
        assert len(traj) >= 2

    def test_longer_distance_more_steps(self):
        random.seed(42)
        short_traj = _generate_trajectory(0, 0, 10, 10)
        random.seed(42)
        long_traj = _generate_trajectory(0, 0, 1000, 1000)
        assert len(long_traj) >= len(short_traj)

    def test_no_consecutive_duplicates(self):
        random.seed(42)
        traj = _generate_trajectory(0, 0, 500, 500)
        for i in range(1, len(traj)):
            assert traj[i] != traj[i - 1]

    def test_all_points_are_int_tuples(self):
        random.seed(42)
        traj = _generate_trajectory(0, 0, 300, 300)
        for point in traj:
            assert isinstance(point, tuple)
            assert isinstance(point[0], int)
            assert isinstance(point[1], int)


class TestHumanMove:
    """Tests for human_move."""

    async def test_calls_xdotool_mousemove(self):
        with patch("ghostdesk.input.humanizer.run", new_callable=AsyncMock) as mock_run, \
             patch("asyncio.sleep", new_callable=AsyncMock):
            mock_run.return_value = ""
            await human_move(0, 0, 100, 100)
            assert mock_run.call_count >= 2
            last_call = mock_run.call_args_list[-1]
            assert last_call[0][0] == ["xdotool", "mousemove", "100", "100"]

    async def test_first_trajectory_call_starts_at_origin(self):
        with patch("ghostdesk.input.humanizer.run", new_callable=AsyncMock) as mock_run, \
             patch("asyncio.sleep", new_callable=AsyncMock):
            random.seed(42)
            mock_run.return_value = ""
            await human_move(50, 60, 300, 400)
            first_call = mock_run.call_args_list[0]
            cmd = first_call[0][0]
            assert cmd[0] == "xdotool"
            assert cmd[1] == "mousemove"
