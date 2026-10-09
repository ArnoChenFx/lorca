package main

import (
	"strings"

	"github.com/egoist/lorca/desktop/model"
	"github.com/egoist/mygo/ui"
)

// The System One sheet, after the provider sheets. System One is the review model that answers how
// likely an action is safe: only Auto-review uses it, and its key, root, and model are the account's,
// shared with every paired Device.

// presentSystemOne opens the System One sheet, filled from the saved service when there is one.
func (w *appWindow) presentSystemOne() {
	s := &systemOneSheet{w: w, baseURL: model.SystemOneServices[0].URL, model: model.DefaultSystemOneModel}
	if saved := store.SystemOne; saved != nil {
		s.editing = true
		s.baseURL = saved.BaseURL
		s.model = saved.Model
	}
	w.present(s.view, func() { s.closed = true })
}

// systemOneSheet is the state of the sheet that connects System One.
type systemOneSheet struct {
	w                   *appWindow
	baseURL, model, key string
	editing             bool
	busy, closed        bool
	status              *providerStatus
}

func (s *systemOneSheet) canConfirm() bool {
	return !s.busy && strings.TrimSpace(s.baseURL) != "" && strings.TrimSpace(s.model) != "" && (s.editing || strings.TrimSpace(s.key) != "")
}

func (s *systemOneSheet) confirm(sh *sheet) {
	if !s.canConfirm() {
		return
	}
	s.busy = true
	s.status = &providerStatus{text: L("Checking the key with %@…", "System One"), tone: model.ToneSecondary, spinning: true}
	store.ConnectSystemOne(model.SystemOneCredentials{BaseURL: s.baseURL, APIKey: s.key, Model: s.model}, func(warning string, err error) {
		if s.closed {
			return
		}
		if err != nil {
			s.busy = false
			s.status = &providerStatus{text: model.ErrorText(err), tone: model.ToneRed}
			return
		}
		if warning != "" {
			s.w.showAlert(alertOptions{Message: L("Saved with a warning"), Informative: warning, Style: alertWarning}, func(int) {
				sh.dismiss()
			})
			return
		}
		sh.dismiss()
	})
}

func (s *systemOneSheet) disconnect(sh *sheet) {
	s.busy = true
	s.status = &providerStatus{text: L("Disconnecting…"), tone: model.ToneSecondary, spinning: true}
	store.DisconnectSystemOne(func(err error) {
		if s.closed {
			return
		}
		if err != nil {
			s.busy = false
			s.status = &providerStatus{text: model.ErrorText(err), tone: model.ToneRed}
			return
		}
		sh.dismiss()
	})
}

func (s *systemOneSheet) view(c *ui.Context, sh *sheet) {
	var leading func()
	if s.editing {
		leading = func() {
			if pushButton(c, L("Disconnect"), pushOptions{Kind: buttonDestructive, Disabled: s.busy}).Clicked() {
				s.disconnect(sh)
			}
		}
	}
	confirm := L("Connect")
	if s.editing {
		confirm = L("Save")
	}
	result := sheetFrame(c, sheetOptions{
		Title:           "System One",
		Subtitle:        L("System One answers how likely an action is safe, for Auto-review only. Encrypted and shared with your paired Devices."),
		Width:           460,
		Confirm:         confirm,
		ConfirmDisabled: !s.canConfirm(),
		Leading:         leading,
	}, func() {
		labelWidth := mcpFormLabelWidth(c, L("Base URL"), L("Model"), L("API key"))
		ui.Column(c).Gap(10).Children(func() {
			providerFormRow(c, labelWidth, L("Base URL"), func() {
				textField(c, &s.baseURL, fieldOptions{Placeholder: model.SystemOneServices[0].URL, Mono: true, Disabled: s.busy, Label: L("Base URL")})
			})
			providerFormNote(c, labelWidth, L("TypeSafe: https://api.typesafe.ai. OpenRouter: https://openrouter.ai/api."), nil)
			providerFormRow(c, labelWidth, L("Model"), func() {
				textField(c, &s.model, fieldOptions{Placeholder: model.DefaultSystemOneModel, Mono: true, Disabled: s.busy, Label: L("Model")})
			})
			providerFormNote(c, labelWidth, L("TypeSafe: jev-latest or jev-1.13.0. OpenRouter: typesafe/jev-1.13 or ~typesafe/jev-latest."), nil)
			providerFormRow(c, labelWidth, L("API key"), func() {
				providerKeyField(c, &s.key, L("API key"), s.busy, !s.editing)
			})
			if s.editing {
				providerFormNote(c, labelWidth, L("Left blank, the saved key is kept."), nil)
			}
		})
		providerStatusLine(c, s.status)
	})
	if result.Confirmed {
		s.confirm(sh)
	}
	if result.Cancelled {
		s.closed = true
		sh.dismiss()
	}
}
