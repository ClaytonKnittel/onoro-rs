'use strict';

const path = require('node:path');

function maybeReplaceSubstr(str, substr, to_replace) {
  const idx = str.indexOf(substr);
  if (idx >= 0) {
    str = to_replace + str.substr(idx + substr.length);
  }
  return str;
}

function stripExt(filename) {
  const idx = filename.indexOf('.');
  return filename.substr(0, idx === -1 ? filename.length : idx);
}

/**
 * This linter rule checks that no relative imports are used (except for
 * associated css module files for React components), and also implements a
 * fixit suggestion to modify relative imports to aliased absolute imports.
 */
module.exports = {
  meta: {
    type: 'suggestion',
    docs: {
      description: 'Description of the rule',
    },
    fixable: 'code',
    schema: [],
  },
  create: function (context) {
    return {
      ImportDeclaration: function (node) {
        if (
          node.source?.type === 'Literal' &&
          node.source.value.startsWith('.')
        ) {
          // Allow relative imports if module stylesheets.
          if (node.source.value.endsWith('.module.css')) {
            return;
          }

          const dir = path.dirname(context.physicalFilename);
          const import_dir = path.resolve(dir, node.source.value);
          let new_import_dir = maybeReplaceSubstr(
            import_dir,
            '/common/src/',
            'common/'
          );
          new_import_dir = maybeReplaceSubstr(
            new_import_dir,
            '/client/src/',
            'client/'
          );
          new_import_dir = maybeReplaceSubstr(
            new_import_dir,
            '/server/src/',
            'server/'
          );

          if (new_import_dir !== import_dir) {
            // Found a substitution to make
            context.report({
              node: node.source,
              message: `Relative imports are disallowed.`,
              fix(fixer) {
                return fixer.replaceText(node.source, `'${new_import_dir}'`);
              },
            });
          } else {
            context.report({
              node: node.source,
              message: `Relative imports are disallowed.`,
            });
          }
        }
      },
    };
  },
};
