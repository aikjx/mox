path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\core\mox-alliance-executor-core\src\dag_engine.rs'

with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

old = '    pub condition: String,'
new = '    pub condition: crate::condition::Condition,'

content = content.replace(old, new)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)

print('done')
