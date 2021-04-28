package server

import (
	"encoding/json"
	"github.com/sirupsen/logrus"
	"net/http"
)

type Description struct {
	Code        int    `json:"code"`
	Description string `json:"description"`
}

type ErrorResult struct {
	Error Description `json:"error"`
}

type MessageResult struct {
	Message Description `json:"message"`
}

func loginHandler(w http.ResponseWriter, r *http.Request) {
	setCorsAllowAll(w)

	err := r.ParseForm()
	if err != nil {
		logrus.Warn("Form parse error: ", err.Error())
	}

	login := r.FormValue("login")
	password := r.FormValue("password")

	if login != "human" || password != "iamnotbot" {
		err := json.NewEncoder(w).Encode(ErrorResult{
			Error: Description{
				Code:        http.StatusUnauthorized,
				Description: "Wrong login or password",
			},
		})
		if err != nil {
			logrus.Warn("ErrorResult encode error: ", err.Error())
		}
		return
	}

	success := json.NewEncoder(w).Encode(MessageResult{
		Message: Description{
			Code:        http.StatusOK,
			Description: "You are successfully logged in!",
		},
	})
	if success != nil {
		logrus.Warn("ErrorResult encode error: ", success.Error())
	}
	return
}
