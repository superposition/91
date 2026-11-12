"""Render Jam Packs with the actual host engine; inspect/import selected MIDI tracks."""
import pathlib

from rust_authoring import call as _author

def midi_from_log(log,output,bpm):
    pathlib.Path(output).write_bytes(bytes(_author("midi-log", log=pathlib.Path(log).read_text(), bpm=bpm)))

def preview(root,pack,out,bars=16,bpm=100,key=0,minor=False,phrases=2):
    return _author("preview", root=str(root), pack=str(pack), output=str(out), bars=bars, bpm=bpm,
                   key=key, minor=minor, phrases=phrases)

def inspect_midi(path):
    return _author("inspect-midi", bytes=list(pathlib.Path(path).read_bytes()))

def import_midi(path,base,track,tonic,start_bar,role,minor=False):
    from fm1_jam import decompile, load
    return _author("import-midi", bytes=list(pathlib.Path(path).read_bytes()), document=decompile(load(base)),
                   track=track, tonic=tonic, bar=start_bar, role=role, minor=minor, name=pathlib.Path(path).name)
