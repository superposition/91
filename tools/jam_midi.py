"""USB-MIDI Jam Pack transport. No firmware flashing or implicit persistent writes."""
import secrets
import time
import rust_codecs as _codecs

from rust_authoring import call as _author

ERRORS = {int(key): value for key, value in _author('device-errors').items()}
FIELDS = _author('device-fields')

def checksum(a): return _codecs.scalar(84, [], [a])
def nibble(a): return list(_codecs.call(90, [], [a]))
def frame(op,token,data=()):
    return _author("device-frame", op=op, token=token, bytes=list(data))

def parse_reply(data,op,token):
    result = _author("device-reply", bytes=list(data), op=op, token=token)
    return None if result is None else bytes(result)

class Device:
    def __init__(self,port=None):
        import mido
        _codecs.executable()  # compile before a device request is pending
        self.mido=mido;self.token=secrets.randbelow(127)+1
        ins=mido.get_input_names();outs=mido.get_output_names()
        self.inp=mido.open_input(_author("device-pick", names=ins, port=port))
        self.out=mido.open_output(_author("device-pick", names=outs, port=port))
    def _run(self, operation, **args):
        plan = _author('device-start', operation=operation, args=args)
        pending = None
        observed_errors = []
        while not plan['done']:
            action = plan['action']
            error = None
            try:
                kind = action['kind']
                if kind == 'request': value = self.request(action['op'], action['data'], action['timeout'])
                elif kind == 'status': value = self.status()
                elif kind == 'read': value = self.read(action['slot'])
                elif kind == 'send': self.out.send(self.mido.Message('sysex', data=action['data'])); value = None
                elif kind == 'clock': value = time.monotonic()
                elif kind == 'sleep': time.sleep(action['seconds']); value = None
                elif kind == 'pending-start': pending = iter(self.inp.iter_pending()); value = None
                elif kind == 'pending-next':
                    message = next(pending, None)
                    value = None if message is None else dict(type=message.type, **({'data': list(message.data)} if message.type == 'sysex' else {}))
                else: raise RuntimeError('unknown device action')
                if isinstance(value, bytes): value = list(value)
            except Exception as observed:
                observed_errors.append(observed)
                error = dict(kind=type(observed).__name__, message=observed.args[0] if isinstance(observed, KeyError) else str(observed))
                value = None
            try:
                plan = _author('device-next', state=plan['state'], value=value, error=error)
            except Exception as returned:
                # Preserve original OS/backend exception objects across an abort
                # transaction. Rust may deliberately replace a protocol error
                # with its explanatory upgrade message.
                for observed in observed_errors:
                    if str(observed) == str(returned): raise observed
                raise
        return plan['value']

    def __enter__(self): return self
    def __exit__(self,*args): self.inp.close();self.out.close()
    def request(self,op,data=(),timeout=3):
        return bytes(self._run("request", op=op, token=self.token, data=list(data), timeout=timeout))
    def capabilities(self):
        return self._run("capabilities")
    def features(self):
        return self._run("features")
    def song_status(self):
        return self._run("song_status")
    def status(self):
        return self._run("status")
    def read(self,slot=0):
        return bytes(self._run("read", slot=slot))
    def audition(self,b):
        return self._run("audition", bytes=list(b))
    def wait_active(self,b,timeout=75):
        return self._run("wait_active", bytes=list(b), timeout=timeout)
    def save(self,slot):
        return self._run("save", slot=slot)
    def control(self,field,value):
        return self._run("control", field=field, value=value)

def ports():
    import mido
    return dict(inputs=mido.get_input_names(),outputs=mido.get_output_names())
