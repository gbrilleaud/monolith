import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:monolith/controller_settings.dart';

void main() {
  testWidgets('reset restores displayed defaults for a selected console', (
    tester,
  ) async {
    int? resetConsole;
    String? resetDevice;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ControllerSettings(
            initialConfig:
                "[[profile]]\nsystem_id = 11\ndevice_id = 'xinput:0'",
            detected: const ['xinput:0 | Manette XInput 0'],
            onSave: (_) async {},
            onReset: (id, device) async {
              resetConsole = id;
              resetDevice = device;
            },
            onReload: () async => '',
          ),
        ),
      ),
    );
    await tester.enterText(
      find.widgetWithText(TextField, 'device_id'),
      'xinput:0',
    );
    await tester.ensureVisible(find.text('RAZ valeurs par défaut'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('RAZ valeurs par défaut'));
    await tester.pumpAndSettle();
    expect(resetConsole, 11);
    expect(resetDevice, 'xinput:0');
    expect(find.text('Manettes par console'), findsOneWidget);
  });
}
