"""Generate isolated synthetic histories for native UI tests; never read user data."""
import json,time,uuid
from pathlib import Path
root=Path('.local-test/v04').resolve();data=root/'runtime';codex=root/'codex';claude=root/'claude'
for p in [data,codex/'sessions',claude/'projects/demo']:p.mkdir(parents=True,exist_ok=True)
settings={'setupComplete':True,'launchAtLogin':False,'deviceName':'Synthetic QA','syncFolder':None,'dockMinutes':60,'dockPreviews':True,'stripLocked':False,'fontScale':120,'theme':'black','meterDisplay':'remaining','codexHome':str(codex),'claudeHome':str(claude),'codexPath':str(root/'missing-codex.exe'),'claudePath':str(root/'missing-claude.exe')}
(data/'settings.json').write_text(json.dumps(settings),encoding='utf-8')
now=int(time.time())
for project in ['Orbit Console','Nebula Tools']:
 session=str(uuid.uuid4());records=[{'type':'session_meta','timestamp':now-1500,'payload':{'id':session,'cwd':'C:/synthetic/'+project,'title':project+' design review'}}]
 for i in range(12):
  turn=str(uuid.uuid4());t=now-1200+i*60;body=['Improve the search experience and keyboard navigation','Review the cache boundaries and performance budget','<send_user_message>Explain this function clearly</send_user_message>','<send_user_message_question_reply>[{"question":"Preferred interval?","answer":"Last hour"}]</send_user_message_question_reply>','Display <script> text safely without executing markup'][i%5]
  records.extend([{'type':'turn_context','timestamp':t,'payload':{'turn_id':turn,'model':'gpt-synthetic','effort':'high','cwd':'C:/synthetic/'+project}},{'type':'event_msg','timestamp':t,'payload':{'type':'user_message','message':body}},{'type':'token_usage_record','timestamp':t+1,'payload':{'turn_id':turn,'response_id':str(uuid.uuid4()),'model':'gpt-synthetic','usage':{'input_tokens':5000,'input_tokens_details':{'cached_tokens':3000},'output_tokens':500,'output_tokens_details':{'reasoning_tokens':100}}}},{'type':'event_msg','timestamp':t+2,'payload':{'type':'task_complete'}}])
 (codex/'sessions'/f'{session}.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in records),encoding='utf-8')
session=str(uuid.uuid4());records=[]
for i in range(12):
 t=now-1400+i*70;pid=str(uuid.uuid4());records.extend([{'type':'user','timestamp':t,'uuid':pid,'sessionId':session,'cwd':'C:/synthetic/Orbit Console','message':{'role':'user','content':'Refine the assistant settings and model selection'}},{'type':'assistant','timestamp':t+1,'parentUuid':pid,'sessionId':session,'message':{'id':str(uuid.uuid4()),'model':'claude-synthetic','usage':{'input_tokens':1000,'cache_read_input_tokens':2000,'output_tokens':300},'stop_reason':'end_turn'}}])
(claude/'projects/demo'/f'{session}.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in records),encoding='utf-8')
print('Synthetic fixture created: 36 prompts, isolated provider homes and disabled executables.')
