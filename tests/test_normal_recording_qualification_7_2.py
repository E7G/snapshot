"""Synthetic negative oracles plus exact scalar hardware receipts; no recapture."""
import copy
import json
from pathlib import Path
import unittest

from normal_recording_qualification_7_2 import long_stored_file_sample_pass


class StoredFileGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report = json.loads(Path(__file__).with_name('mipad2-normal-async-long-7.2-summary.json').read_text())

    def front(self):
        return copy.deepcopy(self.report['front'])

    def test_measured_front_long_sample(self):
        self.assertTrue(long_stored_file_sample_pass(self.front()))

    def test_later_measured_rear_long_sample(self):
        report=json.loads(Path(__file__).with_name('mipad2-normal-rear-long-7.2-summary.json').read_text())
        self.assertTrue(long_stored_file_sample_pass(report['rear']))
        self.assertIs(report['previous_short_rear_and_556ms_normal_failure_preserved'],True)

    def test_rear_pass_not_relabelled_as_thermal_fix_acceptance(self):
        report=json.loads(Path(__file__).with_name('mipad2-normal-rear-long-7.2-summary.json').read_text())
        self.assertIs(report['rear_pass_predates_thermal_guard_patch'],True)
        self.assertIs(report['thermal_guard_causally_proven_to_fix_recording'],False)
        self.assertIs(report['normal_app_overall_qualified'],False)
        self.assertIs(report['default_raw_async_promoted'],False)

    def test_measured_rear_short_sample_not_long_acceptance(self):
        rear = self.report['rear']
        self.assertTrue(rear['metrics']['cadence_gate'])
        self.assertFalse(long_stored_file_sample_pass(rear))

    def test_interruption_rejects_even_long_cadence(self):
        s=self.front();s['recording_completed_requested_interval']=False
        self.assertFalse(long_stored_file_sample_pass(s))

    def test_short_span_rejects_even_claimed_completion(self):
        s=self.front();s['metrics']['pts_span_seconds']=14.66
        self.assertFalse(long_stored_file_sample_pass(s))

    def test_retained_556ms_failure_not_average_fps_pass(self):
        s=self.front();s['metrics']['max_pts_gap_seconds']=.556667;s['metrics']['fps']=30
        self.assertFalse(long_stored_file_sample_pass(s))

    def test_duplicate_pts_not_cadence_boolean_pass(self):
        s=self.front();s['metrics']['nonpositive_pts_intervals']=1
        self.assertFalse(long_stored_file_sample_pass(s))

    def test_decode_must_cover_whole_file(self):
        s=self.front();s['decoded_frames']-=1
        self.assertFalse(long_stored_file_sample_pass(s))

    def test_instrumented_capture_not_normal_untraced_sample(self):
        for key in ('diagnostic_actions','background_capture','private_v4l2_trace'):
            with self.subTest(key=key):
                s=self.front();s[key]=True
                self.assertFalse(long_stored_file_sample_pass(s))

    def test_fault_or_active_pm_reject(self):
        s=self.front();s['kernel_fault_delta']['WARNING']=1
        self.assertFalse(long_stored_file_sample_pass(s))
        s=self.front();s['pm']['isp']['runtime_usage']='1'
        self.assertFalse(long_stored_file_sample_pass(s))

    def test_missing_malformed_and_boolean_numbers_reject(self):
        for value in (None, {}, [], {'metrics':{}}):
            self.assertFalse(long_stored_file_sample_pass(value))
        for key in ('requested_seconds','decoded_frames'):
            s=self.front();s[key]=True
            self.assertFalse(long_stored_file_sample_pass(s))

    def test_nonfinite_metrics_reject(self):
        for value in (float('nan'),float('inf'),-float('inf'),True,'30'):
            for key in ('pts_span_seconds','fps','min_pts_gap_seconds','max_pts_gap_seconds'):
                with self.subTest(value=value,key=key):
                    s=self.front();s['metrics'][key]=value
                    self.assertFalse(long_stored_file_sample_pass(s))

    def test_partial_evidence_never_default_or_oem_promotion(self):
        for key in ('raw_async_default_promoted','normal_app_overall_qualified','android_or_oem_parity'):
            self.assertIs(self.report[key],False)
        self.assertIs(self.report['old_556ms_failure_retained'],True)
        self.assertIs(self.report['previous_hard_freeze_unresolved'],True)
        self.assertIs(self.report['rear_followup']['recording_started'],False)


if __name__=='__main__':
    unittest.main()
