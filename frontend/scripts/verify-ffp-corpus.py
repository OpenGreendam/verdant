#!/usr/bin/env python3
"""Aggregate a user-supplied FFP corpus through the real sequential CLI worker.
Only counts and content digests are retained; vendor presets are not copied.
"""
import argparse, collections, hashlib, json, pathlib, re, subprocess, sys
parser=argparse.ArgumentParser()
parser.add_argument('--binary',required=True,type=pathlib.Path)
parser.add_argument('--corpus',required=True,type=pathlib.Path)
parser.add_argument('--output',required=True,type=pathlib.Path)
args=parser.parse_args()
if args.output.exists():raise SystemExit('output already exists')
families={'FQ4p':'eq','FQ3p':'eq','FQ2p':'eq','FC3p':'compressor','FC2p':'compressor',
'FR2p':'reverb','FPRr':'reverb','FPGr':'gate','FPSr':'deesser','FL2p':'limiter','FS2a':'saturation',
'FSat':'saturation','FSim':'simplon','FMic':'micro','FV3l':'filter','F3Ts':'delay','FPMb':'multiband','FT3i':'synth','FFPr':'one'}
files=sorted(args.corpus.rglob('*.ffp'))
counts={}
identity=hashlib.sha256()
process=subprocess.Popen([str(args.binary),'worker','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
try:
 for index,path in enumerate(files):
  data=path.read_bytes()
  identity.update(hashlib.sha256(data).digest())
  match=re.search(rb'Signature=([^\r\n]+)',data)
  signature=(match[1] if match else data[:4]).decode('ascii')
  if signature not in families:raise ValueError('unknown '+signature)
  req={'protocol':'verdant-cli/1','request_id':str(index),'op':'preset.inspect-ffp',
       'params':{'source':str(path),'device':'verdant.builtin.'+families[signature]}}
  process.stdin.write(json.dumps(req)+'\n');process.stdin.flush()
  reply=json.loads(process.stdout.readline())
  if reply['request_id']!=str(index) or not reply['ok']:raise ValueError(reply)
  report=reply['result']
  row=counts.setdefault(signature,{'recognized':0,'complete':0,'partial':0,'identified_only':0,
   'unsupported_diagnostics':0,'unsupported_by_field':collections.Counter(),'native_result_values':0})
  row['recognized']+=1
  row['complete' if report['complete'] else 'partial' if report['values'] else 'identified_only']+=1
  row['native_result_values']+=len(report['values'])
  row['unsupported_diagnostics']+=len(report['unsupported'])
  for diagnostic in report['unsupported']:
   source=diagnostic.split('：',1)[0]
   source=re.sub(r'Band \d+', 'Band *',source)
   row['unsupported_by_field'][source]+=1
 process.stdin.close()
 if process.wait(timeout=10)!=0:raise ValueError(process.stderr.read())
finally:
 if process.poll() is None:process.kill();process.wait()
receipt={'schema':'verdant-ffp-corpus-proof/1','binary_sha256':hashlib.sha256(args.binary.read_bytes()).hexdigest(),
 'input_set_sha256':identity.hexdigest(),'inputs':len(files),'families':counts}
args.output.parent.mkdir(parents=True,exist_ok=True)
with args.output.open('x',encoding='utf8') as f:json.dump(receipt,f,ensure_ascii=False,indent=2);f.write('\n')
print(json.dumps({'output':str(args.output),'inputs':len(files),'families':{
 s:{k:v for k,v in row.items() if k!='unsupported_by_field'} for s,row in counts.items()}},ensure_ascii=False))
