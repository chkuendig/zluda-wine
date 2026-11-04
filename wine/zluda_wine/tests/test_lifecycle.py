import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('lifecycle_runner',Path(__file__).parent/'abi/run-lifecycle.py')
runner=importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class LifecycleReceipts(unittest.TestCase):
    def test_masked_and_unarmed_setup_failures_are_rejected(self):
        native=['backend-loaded','cuInit','native-dark-query']
        with self.assertRaisesRegex(RuntimeError,'rc=93'):
            runner.validate_case('free-library',93,
                'lifecycle failure55: TlsSetValue(state.tls,state.value) && !setup(&state) (0)\n',
                native,['detach-begin','loader-worker-refused'])
        with self.assertRaisesRegex(RuntimeError,'storage-set=0x0000000000000321'):
            runner.validate_case('free-library',1,
                'LIFECYCLE storage-set=0x0000000000000321\nLIFECYCLE setup-returned=0x0000000000000321\n',
                native,[])

    def test_each_actual_exit_path_needs_complete_ordered_receipt(self):
        for mode in runner.MODES:
            events=[event for event,count in runner.EXPECTED_EVENTS[mode].items() for _ in range(count)]
            pe=runner.EXPECTED_PE_EVENTS[mode]
            runner.validate_case(mode,0,'PASS: lifecycle',events,pe)
            for code,output,native,windows in ((124,'FIXTURE TIMEOUT',events,pe),
                (0,'',events,pe),(0,'PASS:',events[:-1],pe),
                (0,'PASS:',events+['BAD-callback'],pe),(0,'PASS:',events,pe[:-1]),
                (0,'PASS:',events,list(reversed(pe)))):
                with self.subTest(mode=mode,code=code,native=native,windows=windows):
                    with self.assertRaises(RuntimeError):runner.validate_case(mode,code,output,native,windows)
