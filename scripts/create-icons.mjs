// Optional asset regeneration: provide Sharp via GCD_SHARP_MODULE or local install.
// The checked-in icons need no image dependency at build or runtime.
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
const sharp=createRequire(import.meta.url)(process.env.GCD_SHARP_MODULE || 'sharp');
const dir=new URL('../src-tauri/icons/',import.meta.url);mkdirSync(dir,{recursive:true});
const svg=readFileSync(new URL('../public/logo.svg',import.meta.url));
const png=async size=>sharp(svg).resize(size,size).png().toBuffer();
for(const size of [32,128,256])writeFileSync(new URL(`${size}x${size}.png`,dir),await png(size));
writeFileSync(new URL('tray.rgba',dir),await sharp(svg).resize(32,32).ensureAlpha().raw().toBuffer());
const sizes=[16,32,48,64,128,256],frames=await Promise.all(sizes.map(png)),header=Buffer.alloc(6+16*sizes.length);
header.writeUInt16LE(1,2);header.writeUInt16LE(sizes.length,4);let offset=header.length;
for(let i=0;i<sizes.length;i++){const p=6+i*16;header[p]=sizes[i]%256;header[p+1]=sizes[i]%256;header.writeUInt16LE(1,p+4);header.writeUInt16LE(32,p+6);header.writeUInt32LE(frames[i].length,p+8);header.writeUInt32LE(offset,p+12);offset+=frames[i].length;}
writeFileSync(new URL('icon.ico',dir),Buffer.concat([header,...frames]));
const mac=await png(1024),icns=Buffer.alloc(16);icns.write('icns');icns.writeUInt32BE(mac.length+16,4);icns.write('ic10',8);icns.writeUInt32BE(mac.length+8,12);writeFileSync(new URL('icon.icns',dir),Buffer.concat([icns,mac]));
