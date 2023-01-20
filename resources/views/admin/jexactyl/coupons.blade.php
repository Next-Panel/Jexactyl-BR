@extends('layouts.admin')
@include('partials/admin.jexactyl.nav', ['activeTab' => 'coupons'])

@section('title')
Cupons
@endsection

@section('content-header')
    <h1>Cupons<small>Criar e administrar cupons.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Admin</a></li>
        <li class="active">Jexactyl</li>
    </ol>
@endsection

@section('content')
    @yield('jexactyl::nav')
    <form action="{{ route('admin.jexactyl.coupons') }}" method="POST">
        <div class="row">
            <div class="col-xs-12">
                <div class="box @if($enabled) box-success @else box-danger @endif">
                    <div class="box-header with-border">
                        <i class="fa fa-cash"></i>
                        <h3 class="box-title">Sistema de cupons</h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-3">
                                <label for="enabled" class="control-label">Status</label>
                                <select name="enabled" id="enabled" class="form-control">
                                    <option value="1" @if($enabled) selected @endif>Habilitado</option>
                                    <option value="0" @if(!$enabled) selected @endif>Desabilitado</option>
                                </select>
                            </div>
                        </div>
                    </div>
                    <div class="box-footer">
                        {!! csrf_field() !!}
                        <button type="submit" name="_method" value="PATCH" class="btn btn-default pull-right">Salvar</button>
                    </div>
                </div>
            </div>
        </div>
    </form>
    <form action="{{ route('admin.jexactyl.coupons.store') }}" method="POST">
        <div class="row">
            <div class="col-xs-12">
                <div class="box">
                    <div class="box-header">
                        <h3 class="box-title">Criar cupom</h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-3">
                                <label for="code">Código</label>
                                <input type="text" name="code" id="code" class="form-control"/>
                                <small>Um código único para o cupom.</small>
                            </div>
                            <div class="form-group col-md-3">
                                <label for="credits">Créditos</label>
                                <input type="number" name="credits" id="credits" class="form-control"/>
                                <small>A quantidade de créditos a serem concedidos quando resgatados.</small>
                            </div>
                            <div class="form-group col-md-3">
                                <label for="expires">Expira em</label>
                                <input type="number" name="expires" id="expires" class="form-control" value="12"/>
                                <small>A quantidade de tempo em horas até que o cupom expire. Deixe em branco para nunca.</small>
                            </div>
                            <div class="form-group col-md-3">
                                <label for="uses">Máximos Usos</label>
                                <input type="number" name="uses" id="uses" class="form-control" value="1"/>
                                <small>A quantidade máxima de vezes que este cupom pode ser usado.</small>
                            </div>
                        </div>
                    </div>
                    <div class="box-footer">
                        {!! csrf_field() !!}
                        <button type="submit" name="_method" value="POST" class="btn btn-default pull-right">Criar</button>
                    </div>
                </div>
            </div>
        </div>
    </form>
    <div class="row">
        <div class="col-xs-12">
            <div class="box">
                <div class="box-header">
                    <h3 class="box-title">Cupons</h3>
                </div>
                <div class="box-body table-responsive no-padding">
                    <table class="table table-hover">
                        <tbody>
                        <tr>
                            <th>ID</th>
                            <th>Código</th>
                            <th>Créditos</th>
                            <th>Usos Restantes</th>
                            <th>Expira Em</th>
                            <th>Expirado</th>
                        </tr>
                        @foreach($coupons as $coupon)
                            <tr>
                                <td>{{ $coupon->id }}</td>
                                <td>{{ $coupon->code }}</td>
                                <td>{{ $coupon->cr_amount }}</td>
                                <td>{{ $coupon->uses }}</td>
                                <td>{{ $coupon->expires }}</td>
                                <td>@if($coupon->expired) Yes @else No @endif</td>
                            </tr>
                        @endforeach
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>
@endsection
