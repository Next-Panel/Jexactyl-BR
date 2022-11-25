@extends('layouts.admin')
@include('partials/admin.jexactyl.nav', ['activeTab' => 'approvals'])

@section('title')
Aprovações de usuários
@endsection

@section('content-header')
    <h1>Aprovações de usuários<small>Permitir ou negar pedidos de criação de contas.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Admin</a></li>
        <li class="active">Jexactyl</li>
    </ol>
@endsection

@section('content')
    @yield('jexactyl::nav')
    <form action="{{ route('admin.jexactyl.approvals') }}" method="POST">
        <div class="row">
            <div class="col-xs-12">
                <div class="box
                    @if($enabled == 'true')
                        box-success
                    @else
                        box-danger
                    @endif
                ">
                    <div class="box-header with-border">
                        <i class="fa fa-users"></i>
                        <h3 class="box-title">Sistema de Aprovação <small>Decidir se o sistema de aprovação está ativado ou desativado.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Habilitado</label>
                                <div>
                                    <select name="enabled" class="form-control">
                                        <option @if ($enabled == 'false') selected @endif value="false">Desabilitado</option>
                                        <option @if ($enabled == 'true') selected @endif value="true">Habilitado</option>
                                    </select>
                                    <p class="text-muted"><small>Determina se os usuários devem ser aprovados por um administrador para usar o Painel.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label" for="webhook">URL do Webhook</label>
                                <input name="webhook" id="webhook" class="form-control" value="{{ $webhook }}">
                                <p class="text-muted"><small>Fornecer a URL do Webhook Discord para ser usada quando um usuário precisar ser aprovado.</small></p>
                            </div>
                        </div>
                    </div>
                    <div class="box box-footer">
                        {!! csrf_field() !!}
                        <button type="submit" name="_method" value="PATCH" class="btn btn-default pull-right">Salvar mudanças</button>
                    </div>
                </div>
            </div>
        </div>
    </form>
    <div class="row">
        <div class="col-xs-12">
            <div class="box box-success">
                <div class="box-header with-border">
                    <i class="fa fa-list"></i>
                    <h3 class="box-title">Solicitações de aprovação <small>Permitir ou negar solicitações para criar contas.</small></h3>
                    <form id="massapproveform" action="{{ route('admin.jexactyl.approvals.all') }}" method="POST">
                        {!! csrf_field() !!}
                        <button id="approvalAllBtn" class="btn btn-success pull-right">Aprovar todos</button>
                    </form>
                 </div>
                <div class="box-body table-responsive no-padding">
                    <table class="table table-hover">
                        <tbody>
                            <tr>
                                <th>ID do usuário</th>
                                <th>Email</th>
                                <th>Nome do usuário</th>
                                <th>Registrado</th>
                                <th></th>
                                <th></th>
                            </tr>
                            @foreach ($users as $user)
                                <tr>
                                    <td>
                                        <code>{{ $user->id }}</code>
                                    </td>
                                    <td>
                                        {{ $user->email }}
                                    </td>
                                    <td>
                                        <code>{{ $user->username }}</code> ({{ $user->name_first }} {{ $user->name_last }})
                                    </td>
                                    <td>
                                        {{ $user->created_at->diffForHumans() }}
                                    </td>
                                    <td class="text-center">
                                        <form id="approveform" action="{{ route('admin.jexactyl.approvals.approve', $user->id) }}" method="POST">
                                            {!! csrf_field() !!}
                                            <button id="approvalApproveBtn" class="btn btn-xs btn-default">
                                                <i class="fa fa-check text-success"></i>
                                            </button>
                                        </form>
                                    </td>
                                    <td class="text-center">
                                        <form id="denyform" action="{{ route('admin.jexactyl.approvals.deny', $user->id) }}" method="POST">
                                            {!! csrf_field() !!}
                                            <button id="approvalDenyBtn" class="btn btn-xs btn-default">
                                                <i class="fa fa-times text-danger"></i>
                                            </button>
                                        </form>
                                    </td>
                                </tr>
                            @endforeach
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>
@endsection

@section('footer-scripts')
    @parent
    <script>
    $('#approvalDenyBtn').on('click', function (event) {
        event.preventDefault();

        swal({
            type: 'error',
            title: 'Negar este pedido?',
            text: 'Isso removerá esse usuário do painel imediatamente.',
            showCancelButton: true,
            confirmButtonText: 'Negar',
            confirmButtonColor: 'red',
            closeOnConfirm: false
        }, function () {
            $('#denyform').submit()
        });
    });

    $('#approvalApproveBtn').on('click', function (event) {
        event.preventDefault();

        swal({
            type: 'success',
            title: 'Aprovar este pedido?',
            text: 'Este usuário obterá acesso ao painel imediatamente.',
            showCancelButton: true,
            confirmButtonText: 'Aprovar',
            confirmButtonColor: 'green',
            closeOnConfirm: false
        }, function () {
            $('#approveform').submit()
        });
    });

    $('#approvalAllBtn').click(function (event) {
        event.preventDefault();
        swal({
            title: 'Aprovar todos os usuários?',
            text: 'Isso aprovará todos os usuários que aguardam a aprovação.',
            showCancelButton: true,
            confirmButtonText: 'Aprovar tudo',
            confirmButtonColor: 'green',
            closeOnConfirm: false
        }, function () {
            $('#massapproveform').submit()
        });
    });
    </script>
@endsection
