"""Bounded stored-file cadence gate, never optical/live-preview/device acceptance.

Completion is separate from cadence: a 14-second clean MP4 cannot qualify a
requested 60-second recording. Missing facts fail closed, including under -O.
No timestamps, frame hashes, packets or raw device traces are loaded here.
"""
import math


def number(value):
    return type(value) in (int, float) and math.isfinite(value)


def integer(value):
    return type(value) is int and value >= 0


def long_stored_file_sample_pass(sample):
    """Accept one >=60s normal whole-file cadence sample, not a product."""
    try:
        if sample['recording_completed_requested_interval'] is not True:
            return False
        requested = sample['requested_seconds']
        if not number(requested) or requested < 60:
            return False
        m = sample['metrics']
        measured = [m[k] for k in ('pts_span_seconds', 'fps',
                                   'min_pts_gap_seconds', 'max_pts_gap_seconds')]
        if not all(number(x) for x in measured):
            return False
        span, fps, low, high = measured
        if span < requested or not 29.5 <= fps <= 30.5 or not 0 < low <= high <= .05:
            return False
        if m['video_size'] != [1920, 1080] or m['audio_hz'] != 48000 or m['audio_channels'] != 2:
            return False
        if m['guessed_r_frame_rate_used'] is not False:
            return False
        counts = [m[k] for k in ('nonpositive_pts_intervals', 'gaps_above50ms', 'gaps_above100ms')]
        if any(not integer(x) or x != 0 for x in counts):
            return False
        packets = m['packets']
        if not integer(packets) or packets < 1800:
            return False
        if not integer(sample['decoded_frames']) or sample['decoded_frames'] != packets:
            return False
        for k in ('video_and_audio_decode_pass', 'normal_settings_unchanged', 'controls_restored'):
            if sample[k] is not True:
                return False
        for k in ('diagnostic_actions', 'background_capture', 'private_v4l2_trace'):
            if sample[k] is not False:
                return False
        if sample['app_exit'] != dict(MainPID='0', Result='success', ExecMainStatus='0'):
            return False
        faults = sample['kernel_fault_delta']
        required = {'BUG','WARNING','Oops','GPU_HANG','DDR_failure','IUNIT_timeout',
                    'stop_timeout','SP_not_idle','ISP_not_idle'}
        if not required <= faults.keys() or any(not integer(x) or x != 0 for x in faults.values()):
            return False
        pm = sample['pm']
        if set(pm) != {'t4ka3','ov5693','isp'} or any(x != dict(runtime_status='suspended',runtime_usage='0') for x in pm.values()):
            return False
        return True
    except (KeyError, TypeError, ValueError, AttributeError, OverflowError):
        return False
