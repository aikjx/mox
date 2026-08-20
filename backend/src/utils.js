'use strict';

function uid(prefix) {
  return prefix + '_' + Math.random().toString(36).slice(2, 12);
}

module.exports = { uid };
