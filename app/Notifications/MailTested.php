<?php

namespace Jexactyl\Notifications;

use Jexactyl\Models\User;
use Illuminate\Notifications\Notification;
use Illuminate\Notifications\Messages\MailMessage;

class MailTested extends Notification
{
    public function __construct(private User $user)
    {
    }

    public function via(): array
    {
        return ['mail'];
    }

    public function toMail(): MailMessage
    {
        return (new MailMessage())
            ->subject('Mensagem de teste de jexactyl')
            ->greeting('Olá ' . $this->user->name . '!')
            ->line('Este é um teste do sistema de e-mail Jexactyl. Você está pronto para ir!');
    }
}
