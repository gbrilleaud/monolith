import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:monolith/style_settings.dart';
import 'package:monolith/style_theme.dart';

final sampleSnapshot = jsonEncode({
  'catalog': {
    'presets': [
      {'id': 'dark', 'name': 'Sombre'},
      {'id': 'light', 'name': 'Clair'},
    ],
  },
  'selected': 'dark',
  'cartridge_animation': true,
  'resolved': {
    'id': 'dark',
    'name': 'Sombre',
    'colors': {
      'background': '#101820',
      'surface': '#203040',
      'surface_raised': '#304050',
      'primary': '#55AACC',
      'secondary': '#EEAA55',
      'text': '#FAFAFA',
      'text_muted': '#CCCCCC',
      'border': '#445566',
      'error': '#CC3344',
    },
    'font_family': 'mono',
    'density': 0.85,
    'tile_spacing': 14.0,
    'corner_radius': 5.0,
    'background_image': null,
  },
});

void main() {
  test('shared snapshot controls Flutter palette and density', () {
    final style = StyleTheme.fromSnapshot(sampleSnapshot);
    final theme = style.toThemeData();
    expect(theme.scaffoldBackgroundColor, const Color(0xFF101820));
    expect(theme.colorScheme.primary, const Color(0xFF55AACC));
    expect(theme.colorScheme.secondary, const Color(0xFFEEAA55));
    expect(theme.textTheme.bodyMedium?.color, const Color(0xFFFAFAFA));
    expect(theme.extension<StyleTokens>()?.tileSpacing, 14);
    expect(theme.extension<StyleTokens>()?.cornerRadius, 5);
    expect(style.cartridgeAnimation, isTrue);
  });

  testWidgets('preset and cartridge option update independently', (
    tester,
  ) async {
    String current = sampleSnapshot;
    final selections = <String>[];
    final switches = <bool>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StyleSettings(
            snapshotJson: current,
            onSelect: (id) async {
              selections.add(id);
              return current;
            },
            onAnimation: (value) async {
              switches.add(value);
              return current;
            },
            onRefresh: () async => current,
            onSaveCustom: (_) async => current,
          ),
        ),
      ),
    );
    expect(find.text('Animations au lancement des jeux'), findsOneWidget);
    await tester.tap(find.text('Clair'));
    await tester.pump();
    expect(selections, ['light']);
    await tester.tap(find.byType(SwitchListTile));
    await tester.pump();
    expect(switches, [false]);
  });

  testWidgets('preset dupliqué est enregistré avec identifiant custom', (
    tester,
  ) async {
    String? submitted;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StyleSettings(
            snapshotJson: sampleSnapshot,
            onSelect: (_) async => sampleSnapshot,
            onAnimation: (_) async => sampleSnapshot,
            onRefresh: () async => sampleSnapshot,
            onSaveCustom: (json) async {
              submitted = json;
              return sampleSnapshot;
            },
            onChanged: (_) {},
          ),
        ),
      ),
    );
    await tester.ensureVisible(find.text('Enregistrer le thème personnalisé'));
    await tester.pump();
    await tester.tap(find.text('Enregistrer le thème personnalisé'));
    await tester.pump();
    expect(jsonDecode(submitted!)['id'], 'custom');
  });

  testWidgets(
    'actualisation serveur conserve le brouillon JSON non enregistré',
    (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StyleSettings(
              snapshotJson: sampleSnapshot,
              onSelect: (_) async => sampleSnapshot,
              onAnimation: (_) async => sampleSnapshot,
              onRefresh: () async => sampleSnapshot,
              onSaveCustom: (_) async => sampleSnapshot,
            ),
          ),
        ),
      );
      await tester.enterText(
        find.byKey(const Key('style-custom-json')),
        '{"brouillon":true}',
      );
      await tester.ensureVisible(find.text('Actualiser les thèmes du serveur'));
      await tester.tap(find.text('Actualiser les thèmes du serveur'));
      await tester.pump();
      expect(find.byKey(const Key('style-custom-json')), findsOneWidget);
      expect(
        tester
            .widget<TextField>(find.byKey(const Key('style-custom-json')))
            .controller!
            .text,
        '{"brouillon":true}',
      );
    },
  );

  testWidgets('changer de preset ne détruit pas un brouillon JSON', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StyleSettings(
            snapshotJson: sampleSnapshot,
            onSelect: (_) async => sampleSnapshot,
            onAnimation: (_) async => sampleSnapshot,
            onRefresh: () async => sampleSnapshot,
            onSaveCustom: (_) async => sampleSnapshot,
          ),
        ),
      ),
    );
    await tester.enterText(
      find.byKey(const Key('style-custom-json')),
      '{"brouillon":"préserver"}',
    );
    await tester.ensureVisible(find.text('Clair'));
    await tester.tap(find.text('Clair'));
    await tester.pump();
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('style-custom-json')))
          .controller!
          .text,
      '{"brouillon":"préserver"}',
    );
  });

  testWidgets('réinitialisation locale est proposée et répercute le snapshot', (
    tester,
  ) async {
    var resets = 0;
    var updates = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StyleSettings(
            snapshotJson: sampleSnapshot,
            onSelect: (_) async => sampleSnapshot,
            onAnimation: (_) async => sampleSnapshot,
            onRefresh: () async => sampleSnapshot,
            onSaveCustom: (_) async => sampleSnapshot,
            onReset: () async {
              resets++;
              return sampleSnapshot;
            },
            onChanged: (_) => updates++,
          ),
        ),
      ),
    );
    await tester.ensureVisible(find.text('Réinitialiser l’apparence locale'));
    await tester.tap(find.text('Réinitialiser l’apparence locale'));
    await tester.pump();
    expect(resets, 1);
    expect(updates, 1);
  });

  testWidgets('invalid local customization leaves selection unchanged', (
    tester,
  ) async {
    var saved = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StyleSettings(
            snapshotJson: sampleSnapshot,
            onSelect: (_) async => sampleSnapshot,
            onAnimation: (_) async => sampleSnapshot,
            onRefresh: () async => sampleSnapshot,
            onSaveCustom: (_) async {
              saved++;
              throw const FormatException('JSON invalide');
            },
          ),
        ),
      ),
    );
    await tester.enterText(find.byKey(const Key('style-custom-json')), '{bad');
    await tester.ensureVisible(find.text('Enregistrer le thème personnalisé'));
    await tester.pump();
    await tester.tap(find.text('Enregistrer le thème personnalisé'));
    await tester.pump();
    expect(saved, 1);
    expect(find.textContaining('JSON invalide'), findsOneWidget);
    expect(find.text('Sombre'), findsOneWidget);
  });
}
