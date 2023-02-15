
const fs = require('fs');
const Bibliography = require('bibliography');
const StringValue = require('bibliography/internal/bibtex/field_value/StringValue').default;
const immutable = require('immutable');

const entry = fs.readFileSync('./bib/bibliography.bib', 'utf8');
const bib = Bibliography.parseString(entry);

const targetDir = './compiled';

const compileBibEntry = (bibEntry) => {
    let result = '@'+bibEntry['@type'] + '{' + bibEntry._id + ',\n';

    for (const fieldName of Object.keys(bibEntry.fields)) {
        const fieldValue = new StringValue(bibEntry.fields[fieldName])._unicode;
        
        result += '    ' + fieldName + ' = {' + fieldValue + '},\n';
    }

    result += '}\n';

    return result;
}

for (bibEntry of bib.rawEntries) {
    const compiledSource = compileBibEntry(bibEntry);
    const bibIdentifier = bibEntry._id.replace(':', '-').replace('/', '_');

    if (!fs.existsSync(targetDir)) {
        fs.mkdirSync(targetDir, 0744);
    }

    const filepath = targetDir + '/' + bibIdentifier + '.bib';

    console.log('Writing', filepath);
    fs.writeFileSync(filepath, compiledSource, 'utf8');
}
