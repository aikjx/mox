fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\platform\core\mox-plugin-core\src\lifecycle.rs'
with open(fpath, encoding='utf-8-sig', errors='replace') as f:
    c = f.read()

old = '''                | (PluginState::Loaded, PluginState::Unloaded)
                | (PluginState::Initialized, PluginState::Running)'''
new = '''                | (PluginState::Loaded, PluginState::Unloaded)
                | (PluginState::Loaded, PluginState::Stopped)
                | (PluginState::Initialized, PluginState::Running)'''

if old in c:
    c = c.replace(old, new, 1)
    print('Added Loaded -> Stopped transition')
else:
    print('Pattern not found')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(c)
print('Done')
