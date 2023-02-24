'use strict';

const fs = require('fs/promises');
const path = require('path');
const os = require('os');

const Bibliography = require('bibliography');
const StringValue = require('bibliography/internal/bibtex/field_value/StringValue').default;

const compileBibEntry = (bibEntry, ignoredFields) => {
    let result = '@'+bibEntry['@type'] + '{' + bibEntry._id + ','+os.EOL;

    for (const fieldName of Object.keys(bibEntry.fields)) {
        if (ignoredFields.indexOf(fieldName) >= 0) {
            continue;
        }

        const fieldValue = new StringValue(bibEntry.fields[fieldName])._unicode;
        
        result += '    ' + fieldName + ' = {' + fieldValue + '},'+os.EOL;
    }

    result += '}'+os.EOL;

    return result;
}

async function loader(source) {
    if (!'ignoredFields' in this.getOptions()) {
        return source;
    }

    const ignoredFields = this.getOptions().ignoredFields;

    const bib = Bibliography.parseString(source);

    let newSources = [];

    for (const bibEntry of bib.rawEntries) {
        newSources.push(compileBibEntry(bibEntry, ignoredFields));
    }
    return newSources.join(os.EOL);
}

module.exports = loader;
