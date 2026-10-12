import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:monolith/login_view.dart';

void main() {
  testWidgets('connexion locale, serveur modifiable et accès hors ligne', (
    tester,
  ) async {
    String? savedUrl;
    String? credentials;
    var offline = false;
    await tester.pumpWidget(
      MaterialApp(
        home: MonolithLoginView(
          backendUrl: 'http://127.0.0.1:8787',
          onSaveBackendUrl: (value) async {
            savedUrl = value;
          },
          onLogin: (username, password) async {
            credentials = '$username:$password';
          },
          onOffline: () => offline = true,
        ),
      ),
    );
    await tester.enterText(
      find.byKey(const Key('backend-url')),
      'http://192.168.1.39:8788',
    );
    await tester.tap(find.text('Enregistrer le serveur'));
    await tester.pump();
    expect(savedUrl, 'http://192.168.1.39:8788');
    await tester.enterText(find.byKey(const Key('username')), 'Guillaume');
    await tester.enterText(find.byKey(const Key('password')), 'secret');
    await tester.tap(find.text('Connexion locale'));
    await tester.pump();
    expect(credentials, 'Guillaume:secret');
    expect(find.text('secret'), findsNothing);
    await tester.tap(find.text('Continuer hors ligne'));
    expect(offline, isTrue);
  });
}
