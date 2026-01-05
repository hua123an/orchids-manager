const fs = require('fs');
const path = '/Applications/Orchids.app/Contents/Resources/app/main/index.js';
const injectionCode = fs.readFileSync('/Users/huaan/orchids-manager/src-tauri/assets/injection.js', 'utf8');
const targetContent = fs.readFileSync(path, 'utf8');

if (!targetContent.includes('// --- ORCHIDS MANAGER INJECTION START ---')) {
    const newContent = injectionCode + '\n\n' + targetContent;
    fs.writeFileSync(path, newContent);
    console.log('Injected successfully');
} else {
    console.log('Already injected');
}
