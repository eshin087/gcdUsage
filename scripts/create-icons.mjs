// Optional asset regeneration: provide Sharp via GCD_SHARP_MODULE or local install.
// The checked-in icons need no image dependency at build or runtime.
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
const sharp=createRequire(import.meta.url)(process.env.GCD_SHARP_MODULE || 'sharp');
const dir=new URL('../src-tauri/icons/',import.meta.url);mkdirSync(dir,{recursive:true});
const svg=readFileSync(new URL('../public/gcd-logo.png',import.meta.url));
// Optical small-size variant: use the generated mark's g at tray/favicon sizes.
// Three thin letters cannot retain their strokes in a 16-pixel icon.
const source=size=>size<=64 ? sharp(svg).extract({left:110,top:400,width:330,height:440}) : sharp(svg);
const png=async size=>source(size).resize(size,size,{fit:'contain',background:'#000000',kernel:size<=64?'nearest':'lanczos3'}).png().toBuffer();
for(const size of [32,128,256])writeFileSync(new URL(`${size}x${size}.png`,dir),await png(size));
writeFileSync(new URL('tray.rgba',dir),await sharp(await png(32)).ensureAlpha().raw().toBuffer());
const sizes=[16,32,48,64,128,256],frames=await Promise.all(sizes.map(png)),header=Buffer.alloc(6+16*sizes.length);
header.writeUInt16LE(1,2);header.writeUInt16LE(sizes.length,4);let offset=header.length;
for(let i=0;i<sizes.length;i++){const p=6+i*16;header[p]=sizes[i]%256;header[p+1]=sizes[i]%256;header.writeUInt16LE(1,p+4);header.writeUInt16LE(32,p+6);header.writeUInt32LE(frames[i].length,p+8);header.writeUInt32LE(offset,p+12);offset+=frames[i].length;}
writeFileSync(new URL('icon.ico',dir),Buffer.concat([header,...frames]));
const mac=await png(1024),icns=Buffer.alloc(16);icns.write('icns');icns.writeUInt32BE(mac.length+16,4);icns.write('ic10',8);icns.writeUInt32BE(mac.length+8,12);writeFileSync(new URL('icon.icns',dir),Buffer.concat([icns,mac]));
