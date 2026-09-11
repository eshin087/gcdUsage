"""Regenerate the six-page public manual with ReportLab. No user data is read."""
from pathlib import Path
from reportlab.pdfgen import canvas
from reportlab.lib.colors import HexColor
from reportlab.platypus import Paragraph
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.enums import TA_LEFT

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'output/pdf/GCD-Usage-Manual.pdf'
OUT.parent.mkdir(parents=True,exist_ok=True)
W,H=612,792
c=canvas.Canvas(str(OUT),pagesize=(W,H),pageCompression=1)
c.setTitle('GCD Usage | Quick Manual');c.setAuthor('GCD Usage');c.setSubject('Setup, meters, accounting, history, synchronization and limitations')
ink=HexColor('#142136');muted=HexColor('#4b5d73');cyan=HexColor('#216b40');purple=HexColor('#216b40')
body=ParagraphStyle('body',fontName='Helvetica',fontSize=11.5,leading=17,textColor=ink,spaceAfter=10)
small=ParagraphStyle('small',parent=body,fontSize=10,leading=14,textColor=muted)
def text(txt,x,y,width=508,style=body):
 p=Paragraph(txt,style);_,height=p.wrap(width,700);p.drawOn(c,x,y-height);return y-height-12
def header(number,label,title,subtitle):
 c.setFillColor(HexColor('#f7f9fd'));c.rect(0,0,W,H,fill=1,stroke=0)
 c.setFillColor(HexColor('#000000'));c.roundRect(48,718,42,42,3,fill=1,stroke=0)
 c.drawImage(str(ROOT/'src-tauri/icons/128x128.png'),48,718,42,42,mask='auto')
 c.setFillColor(ink);c.setFont('Helvetica-Bold',11);c.drawString(102,742,'GCD USAGE')
 c.setFillColor(muted);c.setFont('Helvetica',9);c.drawRightString(564,742,'QUICK MANUAL  /  v0.5.2 source preview')
 c.setStrokeColor(HexColor('#dbe3ef'));c.line(48,700,564,700)
 c.setFillColor(cyan);c.setFont('Helvetica-Bold',9);c.drawString(48,674,label.upper())
 c.setFillColor(ink);c.setFont('Helvetica-Bold',27);c.drawString(48,634,title)
 y=text(subtitle,48,613,516,small)
 c.setStrokeColor(HexColor('#dbe3ef'));c.line(48,52,564,52)
 c.setFillColor(muted);c.setFont('Helvetica',9);c.drawString(48,34,'GCD Usage  |  Local-first AI activity');c.drawRightString(564,34,f'{number} / 6')
 return y-14
def section(title,txt,y):
 c.setFillColor(purple);c.setFont('Helvetica-Bold',12);c.drawString(48,y,title);return text(txt,48,y-13,516)
def note(title,txt,y):
 p=Paragraph(txt,small);_,h=p.wrap(476,300);height=h+50
 c.setFillColor(HexColor('#eaf1fa'));c.roundRect(48,y-height,516,height,12,fill=1,stroke=0)
 c.setFillColor(cyan);c.setFont('Helvetica-Bold',11);c.drawString(68,y-23,title)
 p.drawOn(c,68,y-height+14);return y-height-18
def finish(y):
 assert y>65,f'Page overflow: {y}';c.showPage()

y=header(1,'Start here','Your AI usage, in focus.','A small desktop companion for Claude Code and Codex / ChatGPT Work activity. No GCD Usage account or hosted service is required.')
y=section('1. Install the matching package','Download Windows x64 or ARM64, or macOS Intel or Apple Silicon, from the repository Releases page. Windows installs for your user. On macOS, drag the app to Applications. These personal preview builds are unsigned or ad-hoc signed; follow the operating system\'s first-launch approval flow only for a download you trust.',y)
y=section('2. Connect your coding tools','Install and sign in to Claude Code and Codex first. GCD Usage checks existing sign-ins. If a connection needs attention, use its reconnect action and follow the provider\'s sign-in flow. Organization SSO depends on the account. GCD Usage does not own or refresh provider credentials.',y)
y=section('3. Let the first import finish','Available local and archived coding-tool logs are imported. Later imports read appended records. Initial import or preview enrichment can temporarily use more resources than normal background operation. Existing token records keep their account attribution.',y)
y=section('4. Make it yours','The default is a cream-on-charcoal terminal theme, percentage left, larger text, an unlocked Windows dock, and a one-hour dock activity interval. Settings groups Appearance, Connections, Sync, and History &amp; advice. Dock size has its own 80-160% slider and right-click presets; its default is 100%. Closing the dashboard keeps the native meters running; Quit ends the app.',y)
y=note('What this app does not see','Simply opening GCD Usage cannot retroactively retrieve ordinary browser chats, browser token counts, or activity on a computer where it has no accessible records. Manual browser imports have been removed; browser activity is outside collection coverage.',y)
finish(y)

y=header(2,'Glance and control','The dock and the dashboard','Windows uses a readable, movable terminal-style dock. macOS uses native menu-bar meters; the Windows hover and duration controls do not apply to macOS.')
y=section('Read the allowance cards','Claude has five-hour, weekly and Fable weekly meters when the provider supplies those windows. Codex shows the available weekly allowance. Fable is its own scoped meter: for example, 18% used becomes 82% left. A missing window stays unavailable; the app never borrows a different pool\'s percentage.',y)
y=section('Change time directly from the dock','Click the Windows token card, or right-click the dock, to choose 30 minutes, 1 hour, 6 hours, 24 hours, 7 days or 30 days. Custom duration opens a themed picker with presets and a whole-number value in minutes, hours or days. The allowed range is 1 minute to 30 days. Apply saves the preference; Cancel keeps the previous value. The total refreshes locally without a model call.',y)
y=section('Hover, scroll, click','Hover a usage card for that provider or the token card for all providers. Each row uses one line for project, prompt preview, time, model and lifetime tokens. Scroll to load older prompts automatically. Multiple models show a compact count; click the row for captured details and full model/reasoning information.',y)
y=section('Understand freshness','Provider polling normally runs every two minutes, with retries delayed after failures or rate limiting. Countdown text is computed locally. A tilde marks stale quota data; an em dash means unavailable and N/A means the reset is unknown. The dashboard provides connection status and manual refresh.',y)
y=note('Two different time scopes','The dock token total follows the selected rolling duration, with no model subtitle. Hover history includes older prompts outside that duration. A prompt row\'s token count is its recorded lifetime total, so adding the rows will not necessarily equal the activity-card total.',y)
finish(y)

y=header(3,'Accounting','What the token numbers mean','Measured token usage and provider allowance percentages are different measurements. Neither should be substituted for the other.')
y=section('Where measurements come from','The app reads recorded Claude Code and Codex model-request usage from local logs. It stores normalized records in local SQLite. Request identifiers help deduplicate repeated records; older cumulative usage counters are handled as deltas. Provider quota snapshots are stored separately.',y)
y=section('The arithmetic','Total tokens = uncached input + cache reads + cache writes + output. Reasoning tokens are displayed separately when available, but are not added again because they overlap output. Null means unknown. Missing token data is never presented as a measured zero.',y)
y=section('Prompts, requests and conversations','A user prompt can cause many model requests and tool loops. These are grouped under the originating prompt where the logs establish the relationship. Attributable subagent work is linked; automatic reviews and background work are distinguished. Model changes remain visible. Historical reasoning that was not recorded stays unknown.',y)
y=section('Intervals and quota estimates','Intervals include the start and exclude the end. A request inside the interval can belong to a prompt started earlier. Remaining allowance falling from 82% to 80% means 2 percentage points consumed. Per-prompt impact is estimated: calibration requires matching account/window snapshots and sufficiently isolated activity. Resets, gaps and overlap can leave impact unknown or unallocated.',y)
y=note('Model advice is advisory','Advice runs locally using allowance, reserve settings and recent statistics. Personalized forecasts require at least 20 completed prompts for a model/effort combination. Sparse data produces provisional guidance. Token statistics are not evidence of model quality, and no model is changed automatically.',y)
finish(y)

y=header(4,'Find your work','Readable, searchable history','Scan coding-tool activity quickly, then open captured details. Browser Chat allowances are reference information only.')
y=section('Find a project, chat or prompt','Coding history shows a project folder name and chat title where source records provide them. Otherwise it shows an unknown label or a short conversation identifier. Only a short preview is retained. Recognized app message wrappers are cleaned; actual code or ordinary markup in your prompt is not treated as executable content.',y)
y=section('Skim a line, open the details','The dashboard history uses one compact line per prompt with time, project, preview, model/effort, tokens and quota impact. Long text is truncated visually. Click the preview for captured chat context and identifiers; expand the row for request-level accounting. A captured preview is not the complete conversation.',y)
y=section('Browser ChatGPT Pro reference','For Pro $200, published caps are 200 GPT-6 Pro messages per week, 170 GPT-5.6 Sol Pro messages per day, and 200 combined per day. The dashboard shows these caps, not a live remaining balance. Browser requests are not visible through Codex. Limits can change; checked September 8, 2026. <link href="https://help.openai.com/en/articles/20001354-gpt-56-and-gpt-6-pro-in-chatgpt">Source: OpenAI Help Center, GPT-5.6 and GPT-6 Pro in ChatGPT.</link>',y)
y=section('Explore and export','Filter coding history by date, provider, model, reasoning and computer. The dashboard supports common ranges, calendar periods and custom dates. CSV export contains recorded previews and measurements; formula-like text is neutralized. Treat exports as sensitive conversation data.',y)
y=note('If a preview is incomplete','Old previews may already be truncated. Improvement requires the original local log. The app cannot recreate deleted or inaccessible conversations. Retired browser-import records are preserved for compatibility but excluded from the dock.',y)
finish(y)

y=header(5,'Across devices','Sync through a shared folder','Synchronization combines captured records. It is not browser monitoring or automatic access to another device\'s provider account.')
y=section('Set up the same logical folder','Choose a folder that is actually synchronized between your computers by an approved file-sync service. Configure that corresponding local folder in GCD Usage on each installed device, give devices recognizable names, and keep app versions updated. A Google Drive website link alone is not a local synchronized folder.',y)
y=section('What travels between devices','The app exchanges versioned event batches containing prompt previews, project/chat labels and usage metadata. Merges are idempotent, so duplicate delivery does not multiply usage. Each device keeps its own SQLite database. Provider credentials, raw logs and source-device file paths are not synchronized.',y)
y=section('Check delivery, not just a folder check','Settings shows the most recent check, pending records and delivery status reported by other devices. A successful local folder check is not proof that another computer received the files. Offline devices catch up after the folder service delivers records and GCD Usage runs again.',y)
y=section('When the work computer is restricted','Without an installed collector or accessible local coding logs, the app cannot ingest ordinary browser chats or their tokens. Google Drive in a browser does not provide a shared local folder. There is no manual upload flow, browser monitoring or app-managed cloud sync.',y)
y=note('The shared folder is a trust boundary','Anyone who can read the sync files may read previews and context labels. Writers can introduce records. Use access-controlled storage and trusted devices. GCD Usage does not add end-to-end encryption or an authenticated cloud identity. Do not select a broadly shared folder.',y)
finish(y)

y=header(6,'Care and limits','Keep it small and reliable','The installed app is measured in megabytes. Build tools and compiler caches belong to development and can temporarily occupy gigabytes.')
y=section('Resource use','Native meters avoid a resident dashboard webview. The dashboard is created when opened and released when closed. The visual redesign uses static surfaces, not animated graphics or background model calls. Imports, sign-in helpers and provider checks can create brief memory/CPU peaks; idle measurements are not peak measurements.',y)
y=section('Security and privacy','The local dashboard has narrowly scoped commands, restricted navigation and bounded external reads. Conversation links are reconstructed from validated identifiers. Credentials are not copied into history or sync. This is a reviewed personal preview, not a guarantee against every vulnerability or a replacement for operating-system updates.',y)
y=section('Troubleshooting','Stale or unavailable meter: check the provider sign-in and connection message, then refresh after any retry delay. Missing history: confirm the coding tool produced local logs. Missing project/title: the source may not record it. Sync delay: check both the sync service and the receiving app\'s delivery report.',y)
y=section('Updates, removal and retained data','Updates are manual: install the newer matching release. Keep a protected backup of important history before changing machines. Do not synchronize the database itself. Uninstall removes the application; retained local history and exported/shared files may remain and require a separate deliberate cleanup. Never delete original provider logs to shrink a development folder.',y)
y=note('Release notes and support','Find current downloads, validation notes, known limitations and the security reporting policy in the repository. Provider interfaces and model allowances can change. Report app version, platform and a sanitized description; do not attach credentials or private logs to a public issue.',y)
text('<link href="https://github.com/eshin087/gcdUsage">github.com/eshin087/gcdUsage</link>',48,y,516,small)
finish(y)
c.save()
print(OUT)
