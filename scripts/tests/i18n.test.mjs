import test from 'node:test';
import assert from 'node:assert/strict';
import { catalogs, DEFAULT_LANGUAGE, LANGUAGE_STORAGE_KEY, readLanguage, translate } from '../../src/i18n-core.ts';

test('first launch and invalid or blocked storage default to English', () => {
  assert.equal(DEFAULT_LANGUAGE, 'en');
  assert.equal(readLanguage(), 'en');
  for (const value of [null, 'broken', JSON.stringify('el'), JSON.stringify({ language: 'pt-BR' })]) {
    assert.equal(readLanguage({ getItem: () => value }), 'en');
  }
  assert.equal(readLanguage({ getItem() { throw Error('storage blocked'); } }), 'en');
});
test('a valid saved language is restored', () => {
  for (const language of ['en', 'pt-BR']) {
    assert.equal(readLanguage({ getItem(key) { assert.equal(key, LANGUAGE_STORAGE_KEY); return JSON.stringify(language); } }), language);
  }
});
test('catalogs have the same keys, non-empty values and matching placeholders', () => {
  assert.deepEqual(Object.keys(catalogs.en).sort(), Object.keys(catalogs['pt-BR']).sort());
  const params = (value) => [...value.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
  for (const [key, value] of Object.entries(catalogs.en)) {
    assert.ok(value.trim(), key);
    assert.ok(catalogs['pt-BR'][key].trim(), key);
    assert.deepEqual(params(value), params(key), key);
    assert.deepEqual(params(catalogs['pt-BR'][key]), params(key), key);
  }
});
test('labels, interpolated messages and backend diagnostics use the selected language', () => {
  assert.equal(translate('en', 'Brilho'), 'Brightness');
  assert.equal(translate('pt-BR', 'Brilho'), 'Brilho');
  assert.equal(translate('en', 'Perfil {profile} ativo', { profile: 2 }), 'Profile 2 active');
  assert.equal(translate('en', 'Perfil 3 ativo'), 'Profile 3 active');
  assert.equal(translate('en', 'valor inválido: estágio deve ser 1..5'), 'Invalid value: Stage must be between 1 and 5');
  assert.equal(translate('pt-BR', 'unknown lighting effect'), 'efeito de iluminação desconhecido');
  assert.equal(translate('en', 'USB: this model: este modelo usa um protocolo que ainda não foi implementado'), 'USB: this model: this model uses a protocol that is not implemented yet');
});
test('unknown hardware/OS text is preserved, and parameters are literal', () => {
  assert.equal(translate('en', '/dev/hidraw4: permission denied'), '/dev/hidraw4: permission denied');
  assert.equal(translate('en', 'toString'), 'toString');
  assert.equal(translate('en', 'Perfil {profile}', { profile: '$1 <board>' }), 'Profile $1 <board>');
});
